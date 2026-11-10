//! Attribute macro declaring an xfconf-backed settings object.

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Data, DataStruct, DeriveInput, Expr, Field, Fields, LitStr, Meta, Type, parse_macro_input,
};

/// Glib plumbing of one xfconf-backed field.
struct Class {
    gtype: TokenStream2,
    /// Concrete `ParamSpecXxx` type owning the builder.
    spec: Ident,
    /// `Some(&default)` for `ParamSpecString`.
    string: bool,
    /// Field type is `Option<String>`.
    opt: bool,
}

fn classify(ty: &Type) -> Option<Class> {
    let norm: String = ty
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let stringish = |s: &str| {
        s == "String"
            || s.ends_with("::String")
            || s.ends_with("::std::string::String")
            || s.ends_with("::alloc::string::String")
    };
    let class = |gtype: TokenStream2, spec: &str, string: bool, opt: bool| {
        Some(Class {
            gtype,
            spec: Ident::new(spec, ty_span()),
            string,
            opt,
        })
    };
    match norm.as_str() {
        "bool" => class(
            quote!(::xfce_panel::glib::Type::BOOL),
            "ParamSpecBoolean",
            false,
            false,
        ),
        "i32" => class(
            quote!(::xfce_panel::glib::Type::I32),
            "ParamSpecInt",
            false,
            false,
        ),
        "u32" => class(
            quote!(::xfce_panel::glib::Type::U32),
            "ParamSpecUInt",
            false,
            false,
        ),
        "i64" => class(
            quote!(::xfce_panel::glib::Type::I64),
            "ParamSpecInt64",
            false,
            false,
        ),
        "f64" => class(
            quote!(::xfce_panel::glib::Type::F64),
            "ParamSpecDouble",
            false,
            false,
        ),
        "Option<String>" => class(
            quote!(::xfce_panel::glib::Type::STRING),
            "ParamSpecString",
            true,
            true,
        ),
        _ if stringish(&norm) => class(
            quote!(::xfce_panel::glib::Type::STRING),
            "ParamSpecString",
            true,
            false,
        ),
        _ if norm.starts_with("Option<") && stringish(norm[7..].trim_end_matches('>')) => class(
            quote!(::xfce_panel::glib::Type::STRING),
            "ParamSpecString",
            true,
            true,
        ),
        _ => None,
    }
}

fn ty_span() -> proc_macro2::Span {
    proc_macro2::Span::call_site()
}

/// `path` and `default` of `#[xfconf(path = "/x", default = false)]`.
fn xfconf_attr(field: &Field) -> syn::Result<Option<(LitStr, Expr)>> {
    let mut path: Option<LitStr> = None;
    let mut default: Option<Expr> = None;
    for attr in &field.attrs {
        let Meta::List(list) = &attr.meta else {
            continue;
        };
        if !list.path.is_ident("xfconf") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("path") {
                if path.is_some() {
                    return Err(meta.error("xfconf: `path` given twice"));
                }
                path = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("default") {
                if default.is_some() {
                    return Err(meta.error("xfconf: `default` given twice"));
                }
                default = Some(meta.value()?.parse::<Expr>()?);
            } else {
                return Err(meta.error("xfconf: expected `path` or `default`"));
            }
            Ok(())
        })?;
    }
    match (path, default) {
        (Some(path), Some(default)) => Ok(Some((path, default))),
        (Some(_), None) => Err(syn::Error::new_spanned(
            field,
            "xfconf: `default` is required (xfconf returns NULL until the property exists)",
        )),
        (None, Some(_)) => Err(syn::Error::new_spanned(field, "xfconf: `path` is required")),
        (None, None) => Ok(None),
    }
}

/// `panel_settings(name = Renamed)`
struct MacroArgs {
    name: Option<Ident>,
}

impl syn::parse::Parse for MacroArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        use syn::Token;
        let mut name = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if key == "name" {
                name = Some(input.parse::<Ident>()?);
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "panel_settings: expected `name`",
                ));
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(MacroArgs { name })
    }
}

#[proc_macro_attribute]
pub fn panel_settings(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as MacroArgs);
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input, &args)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: &DeriveInput, args: &MacroArgs) -> syn::Result<TokenStream2> {
    let name = args.name.clone().unwrap_or_else(|| input.ident.clone());
    let vis = &input.vis;
    let name_str = name.to_string();
    let attrs = &input.attrs;

    let Data::Struct(DataStruct {
        fields: Fields::Named(named),
        ..
    }) = &input.data
    else {
        return Err(syn::Error::new_spanned(
            input,
            "panel_settings: expected a struct with named fields",
        ));
    };

    let mut wrapper_attrs = Vec::new();
    for attr in attrs {
        if let Meta::List(list) = &attr.meta
            && list.path.is_ident("xfconf")
        {
            return Err(syn::Error::new_spanned(
                attr,
                "panel_settings: `xfconf` belongs on a field",
            ));
        }
        wrapper_attrs.push(attr);
    }

    let mut imp_fields = Vec::new();
    let mut imp_defaults = Vec::new();
    let mut specs = Vec::new();
    let mut get_arms = Vec::new();
    let mut set_arms = Vec::new();
    let mut binds = Vec::new();
    let mut accessors = Vec::new();

    for field in &named.named {
        let ident = field.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(field, "panel_settings: expected a named field")
        })?;
        let ty = &field.ty;
        let Some(xfconf) = xfconf_attr(field)? else {
            let field_attrs = &field.attrs;
            imp_fields.push(quote!(#(#field_attrs)* pub #ident: #ty));
            imp_defaults.push(quote!(#ident: ::std::default::Default::default()));
            continue;
        };
        let (path, default) = xfconf;
        let Some(class) = classify(ty) else {
            return Err(syn::Error::new_spanned(
                ty,
                "panel_settings: unsupported property type (bool, i32, u32, i64, f64, String, Option<String>)",
            ));
        };
        let Class {
            gtype,
            spec,
            string,
            opt,
        } = class;

        let prop = ident.to_string().replace('_', "-");
        let nick = ident.to_string();
        let path_str = path.value();
        let blurb = format!("xfce-panel: {path_str}");
        let getter = ident.clone();
        let setter = format_ident!("set_{ident}");
        let notify = format_ident!("notify_{ident}");
        let connect_notify = format_ident!("connect_{ident}_notify");
        let field_attrs: Vec<_> = field
            .attrs
            .iter()
            .filter(|a| !a.path().is_ident("xfconf"))
            .collect();

        imp_fields.push(quote!(#(#field_attrs)* pub #ident: RefCell<#ty>));
        let stored = if string {
            let conv = quote!(::std::string::String::from);
            if opt {
                quote!(::std::option::Option::Some(#conv(#default)))
            } else {
                quote!(#conv(#default))
            }
        } else {
            quote!(#default)
        };
        imp_defaults.push(quote!(#ident: RefCell::new(#stored)));

        let spec_value = if string {
            quote!({
                let default = ::std::string::String::from(#default);
                ::xfce_panel::glib::#spec::builder(#prop)
                    .nick(#nick)
                    .blurb(#blurb)
                    .default_value(::std::option::Option::Some(default.as_str()))
                    .build()
            })
        } else {
            quote!(
                ::xfce_panel::glib::#spec::builder(#prop)
                    .nick(#nick)
                    .blurb(#blurb)
                    .default_value(#default)
                    .build()
            )
        };
        specs.push(quote!(specs.push(#spec_value);));

        get_arms.push(quote!(
            if pspec.name() == #prop {
                return ::xfce_panel::glib::value::ToValue::to_value(&*self.#ident.borrow());
            }
        ));
        set_arms.push(quote!(
            if pspec.name() == #prop {
                *self.#ident.borrow_mut() = ::xfce_panel::glib::Value::get::<#ty>(value).unwrap_or_default();
                return;
            }
        ));
        binds.push(quote!(
            channel.bind(#path, #gtype, self, #prop);
            channel.seed_default(self, #path, #prop);
            if let ::std::option::Option::Some(stored) = channel.get::<::xfce_panel::glib::Value>(#path) {
                if let ::std::result::Result::Ok(v) = ::xfce_panel::glib::Value::get::<#ty>(&stored) {
                    *::xfce_panel::glib::subclass::prelude::ObjectSubclassIsExt::imp(self)
                        .#ident
                        .borrow_mut() = v;
                }
            }
        ));

        let doc = format!(" Current value of the `{path_str}` GObject property.");
        let doc_set = format!(
            " Writes the `{path_str}` GObject property: `notify` triggers the xfconf save."
        );
        let doc_notify = format!(" Emits `notify` on the `{path_str}` GObject property.");
        let doc_connect = format!(" Listens to changes of the `{path_str}` GObject property.");
        accessors.push(quote!(
            #[doc = #doc]
            #vis fn #getter(&self) -> #ty {
                ::xfce_panel::glib::subclass::prelude::ObjectSubclassIsExt::imp(self).#ident.borrow().clone()
            }

            #[doc = #doc_set]
            #vis fn #setter(&self, value: #ty) {
                ::xfce_panel::glib::prelude::ObjectExt::set_property(self, #prop, &value);
            }

            #[doc = #doc_notify]
            #vis fn #notify(&self) {
                ::xfce_panel::glib::prelude::ObjectExt::notify(self, #prop);
            }

            #[doc = #doc_connect]
            #vis fn #connect_notify(
                &self,
                f: impl ::std::ops::Fn(&Self) + 'static,
            ) -> ::xfce_panel::glib::SignalHandlerId {
                ::xfce_panel::glib::prelude::ObjectExt::connect_notify_local(
                    self,
                    ::std::option::Option::Some(#prop),
                    move |obj, _| f(obj),
                )
            }
        ));
    }

    let imp_name = format_ident!("imp");

    Ok(quote! {
        ::xfce_panel::glib::wrapper! {
            #(#wrapper_attrs)*
            #vis struct #name(ObjectSubclass<#imp_name::#name>);
        }

        #[allow(non_snake_case, unused, clippy::all)]
        mod #imp_name {
            use ::xfce_panel::glib;
            use ::std::cell::RefCell;
            use ::xfce_panel::glib::subclass::prelude::*;

            #[derive(::std::fmt::Debug)]
            pub struct #name {
                #(#imp_fields,)*
            }

            impl ::std::default::Default for #name {
                fn default() -> Self {
                    Self {
                        #(#imp_defaults,)*
                    }
                }
            }

            #[::xfce_panel::glib::object_subclass]
            impl ::xfce_panel::glib::subclass::prelude::ObjectSubclass for #name {
                const NAME: &'static str = concat!("XfcePanelRs", #name_str);
                type Type = super::#name;
                type ParentType = ::xfce_panel::glib::Object;
            }

            impl ::xfce_panel::glib::subclass::prelude::ObjectImpl for #name {
                fn properties() -> &'static [::xfce_panel::glib::ParamSpec] {
                    static PROPERTIES: ::std::sync::OnceLock<
                        ::std::vec::Vec<::xfce_panel::glib::ParamSpec>,
                    > = ::std::sync::OnceLock::new();
                    PROPERTIES.get_or_init(|| {
                        use ::xfce_panel::glib::prelude::ParamSpecBuilderExt;
                        let mut specs = ::std::vec::Vec::new();
                        #(#specs)*
                        specs
                    })
                }

                fn property(
                    &self,
                    _id: usize,
                    pspec: &::xfce_panel::glib::ParamSpec,
                ) -> ::xfce_panel::glib::Value {
                    #(#get_arms)*
                    unimplemented!("xfce-panel: unknown property {}", pspec.name())
                }

                fn set_property(
                    &self,
                    _id: usize,
                    value: &::xfce_panel::glib::Value,
                    pspec: &::xfce_panel::glib::ParamSpec,
                ) {
                    #(#set_arms)*
                    unimplemented!("xfce-panel: unknown property {}", pspec.name())
                }

                fn constructed(&self) {
                    self.parent_constructed();
                }
            }
        }

        impl #name {
            /// Builds the object.
            #vis fn new() -> Self {
                ::xfce_panel::glib::object::Object::new()
            }

            /// Binds every property to the plugin's xfconf namespace: the
            /// `ParamSpec` default is written when the property is missing, the
            /// current value is read back, and the GObject ↔ xfconf binding is
            /// set up.
            #vis fn bind_to(&self, channel: &::xfce_panel::Channel) {
                #(#binds)*
            }

            #(#accessors)*
        }
    })
}
