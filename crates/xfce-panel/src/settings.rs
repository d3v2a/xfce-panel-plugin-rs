//! Plugin settings persisted in xfconf, the standard Xfce configuration store.
//!
//! The canonical plugin pattern (what `xfce4-panel`'s own plugins do through the
//! private `panel_properties_bind`, `common/panel-xfconf.c:89`) is to bind `GObject`
//! properties to xfconf properties under the plugin's property base
//! (`/plugins/plugin-<id>`). Values then save on change, propagate to every
//! process watching the channel, and survive a panel restart —
//! [`PanelPlugin::save`](crate::PanelPlugin::save) stays a no-op.
//!
//! ```ignore
//! let settings = MySettings::new(); // a glib::Object with ParamSpec properties
//! if let Some(channel) = Channel::new(plugin) {
//!     channel.bind("/show-seconds", glib::Type::BOOL, &settings, "show-seconds");
//! }
//! ```

use std::ffi::CStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use glib::prelude::*;

use xfce_panel_sys::panel::xfce_panel_get_channel_name;

use crate::Plugin;

/// xfconf reference count taken by the channels opened in this process.
/// `xfconf_init`/`xfconf_shutdown` are refcounted (`xfconf.c:36`), and the
/// panel process already holds a reference: ours must stay balanced.
static REFCOUNT: AtomicUsize = AtomicUsize::new(0);

/// The xfconf channel of one plugin instance, restricted to its property base.
///
/// Non-singleton channel (`xfconf_channel_new_with_property_base`,
/// `xfconf-channel.c:581`), so each plugin instance owns its own cache.
///
/// Dropping it unbinds every property bound through this channel and releases
/// the xfconf reference taken at construction — do it from
/// [`PanelPlugin::free_data`](crate::PanelPlugin::free_data) by keeping the
/// channel in the plugin state.
#[derive(Debug)]
pub struct Channel {
    inner: Option<xfconf::Channel>,
}

impl Channel {
    /// Opens the channel of `plugin`: channel `xfce4-panel`, property base
    /// `/plugins/plugin-<id>`.
    ///
    /// Returns `None` when xfconf cannot start (no session bus): the plugin
    /// must then run on its compiled-in defaults.
    pub fn new(plugin: &Plugin) -> Option<Self> {
        if REFCOUNT.fetch_add(1, Ordering::SeqCst) == 0 && xfconf::init().is_err() {
            REFCOUNT.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        let name = unsafe { CStr::from_ptr(xfce_panel_get_channel_name()) }.to_string_lossy();
        Some(Self {
            inner: Some(xfconf::Channel::with_property_base(
                &name,
                &plugin.property_base(),
            )),
        })
    }

    /// Opens an arbitrary channel: `name` is the xfconf channel, `property_base`
    /// the absolute prefix every relative path is resolved against.
    ///
    /// This is what [`Channel::new`] builds from a plugin
    /// (`xfce4-panel` + `/plugins/plugin-<id>`, `common/panel-private.h:83`);
    /// taking it explicitly is what makes the settings usable outside a plugin
    /// (tests, standalone tools).
    pub fn with_base(name: &str, property_base: &str) -> Option<Self> {
        if REFCOUNT.fetch_add(1, Ordering::SeqCst) == 0 && xfconf::init().is_err() {
            REFCOUNT.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(Self {
            inner: Some(xfconf::Channel::with_property_base(name, property_base)),
        })
    }

    /// Writes `value` into xfconf when `property` is not set yet, and reports
    /// whether it wrote.
    ///
    /// A binding never creates the xfconf property (the C binding only seeds
    /// when the panel passes `save=TRUE`, `common/panel-xfconf.c:26`), so
    /// without this a fresh plugin has nothing in `xfconf-query -l` and the
    /// defaults stay invisible.
    ///
    /// Existence is checked untyped ([`Channel::exists`]): a stored value of
    /// another type counts as set and is left alone.
    pub fn ensure<V>(&self, property: &str, value: V) -> bool
    where
        V: xfconf::ToXfconfValue,
    {
        if self.exists(property) {
            false
        } else {
            self.set(property, value)
        }
    }

    /// Binds a property of the plugin's xfconf namespace to a `GObject` property.
    ///
    /// `xfconf_property` is relative to the plugin's property base and must
    /// start with `/` — the channel prefixes it verbatim
    /// (`REAL_PROP`, `xfconf-channel.c:44`), so `"/show-seconds"` is
    /// `/plugins/plugin-<id>/show-seconds`.
    ///
    /// Non-destructive: an existing xfconf value overwrites the object
    /// property, a missing one leaves it untouched
    /// (`xfconf-binding.c:465`). Returns `None` (no binding) when the path is
    /// not absolute, which `xfconf_g_property_bind` rejects
    /// (`xfconf-binding.c:549`).
    ///
    /// Deleting the xfconf property later resets the object property to its
    /// `ParamSpec` default (`xfconf-binding.c:376`), which is `NULL` for
    /// strings: bind `Option<String>` properties, never `String` — the derived
    /// setter panics on `NULL`, inside the panel process.
    pub fn bind<O: IsA<glib::Object>>(
        &self,
        xfconf_property: &str,
        xfconf_property_type: glib::Type,
        object: &O,
        object_property: &str,
    ) -> Option<xfconf::BindingId> {
        let channel = self.inner.as_ref()?;
        if !xfconf_property.starts_with('/') {
            return None;
        }
        Some(xfconf::ChannelBindingExt::bind_property(
            channel,
            xfconf_property,
            xfconf_property_type,
            object,
            object_property,
        ))
    }

    /// `get`/`set` helpers on the plugin's namespace; `property` follows the
    /// same relative-path rule as [`Channel::bind`].
    #[must_use]
    pub fn get<V: xfconf::TryFromXfconfValue>(&self, property: &str) -> Option<V> {
        self.inner
            .as_ref()
            .and_then(|c| xfconf::ChannelExtManual::get_property(c, property))
    }

    /// See [`Channel::get`].
    pub fn set<V: xfconf::ToXfconfValue>(&self, property: &str, value: V) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|c| xfconf::ChannelExtManual::set_property(c, property, value))
    }

    /// Watches xfconf changes under the plugin's namespace. The `property`
    /// handed to the callback is relative to the base
    /// (`xfconf-channel.c:392`).
    ///
    /// # Panics
    ///
    /// Panics when the channel failed to open (`inner` is `None`), which only
    /// happens when xfconf is unavailable (no session bus).
    pub fn connect_property_changed<F>(&self, property: Option<&str>, f: F) -> glib::SignalHandlerId
    where
        F: Fn(&xfconf::Channel, &str, Option<&glib::Value>) + Send + Sync + 'static,
    {
        self.inner
            .as_ref()
            .unwrap()
            .connect_property_changed(property, f)
    }
}

/// The xfconf type of a Rust setting type, for
/// [`panel_settings`](crate::panel_settings).
///
/// The mapping is the one `xfconf_g_property_bind` needs as `property_type`
/// (`xfconf-binding.c:549`). `String` and `Option<String>` share `STRING`;
/// prefer `Option<String>` ([`Channel::bind`](Channel::bind)).
///
/// ```rust
/// use xfce_panel::XfconfType;
///
/// assert_eq!(bool::TYPE, glib::Type::BOOL);
/// assert_eq!(i32::TYPE, glib::Type::I32);
/// assert_eq!(f64::TYPE, glib::Type::F64);
/// assert_eq!(Option::<String>::TYPE, glib::Type::STRING);
/// ```
pub trait XfconfType {
    /// The `GType` the xfconf property is declared with.
    const TYPE: glib::Type;
}

macro_rules! impl_xfconf_type {
    ($($rust:ty => $ty:ident),* $(,)?) => {$(
        impl XfconfType for $rust {
            const TYPE: glib::Type = glib::Type::$ty;
        }
    )*};
}

impl_xfconf_type! {
    bool => BOOL,
    i32 => I32,
    u32 => U32,
    i64 => I64,
    f64 => F64,
    String => STRING,
    Option<String> => STRING,
}

impl Channel {
    /// Whether `property` exists in xfconf, whatever its stored type.
    ///
    /// Untyped on purpose: the typed getters of the xfconf crate log a
    /// `xfconf-WARNING` on every failed conversion, and a probe is not an
    /// error (`xfconf_g_property_get` semantics, `xfconf-channel.c:1006`).
    #[must_use]
    pub fn exists(&self, property: &str) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|c| xfconf::ChannelExtManual::get_property_value(c, property).is_some())
    }

    /// Writes the `ParamSpec` default of `object_property` into xfconf when the
    /// property is absent — the `save = TRUE` half of `panel_properties_bind`
    /// (`common/panel-xfconf.c:26`). Without it a fresh plugin has nothing in
    /// `xfconf-query -l`: the binding never creates the property.
    pub fn seed_default<O: IsA<glib::Object>>(
        &self,
        object: &O,
        property: &str,
        object_property: &str,
    ) {
        if self.exists(property) {
            return;
        }
        let Some(pspec) = object.find_property(object_property) else {
            return;
        };
        let value = pspec.default_value();
        match pspec.value_type() {
            glib::Type::BOOL => {
                self.set(property, value.get::<bool>().unwrap_or_default());
            }
            glib::Type::I32 => {
                self.set(property, value.get::<i32>().unwrap_or_default());
            }
            glib::Type::U32 => {
                self.set(property, value.get::<u32>().unwrap_or_default());
            }
            glib::Type::I64 => {
                self.set(property, value.get::<i64>().unwrap_or_default());
            }
            glib::Type::F64 => {
                self.set(property, value.get::<f64>().unwrap_or_default());
            }
            glib::Type::STRING => {
                let text = value.get::<Option<String>>().unwrap_or_default();
                self.set(property, text.unwrap_or_default());
            }
            _ => {}
        }
    }

    /// Unbinds every property bound through this channel.
    pub fn unbind_all(&self) {
        if let Some(channel) = &self.inner {
            xfconf::ChannelBindingExt::unbind_all(channel);
        }
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        if let Some(channel) = self.inner.take() {
            xfconf::ChannelBindingExt::unbind_all(&channel);
            drop(channel);
            if REFCOUNT.fetch_sub(1, Ordering::SeqCst) == 1 {
                unsafe { xfconf::shutdown() };
            }
        }
    }
}
