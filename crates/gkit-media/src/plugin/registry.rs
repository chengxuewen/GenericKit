use gkit_core::plugin::loader::PluginLib;

use std::collections::HashMap;
use std::sync::RwLock;

pub type FactoryFn<T> = Box<dyn Fn() -> T + Send + Sync>;

pub struct PluginEntry<T> {
    pub name: String,
    factory: FactoryFn<T>,
    _lib: Option<PluginLib>,
}

pub struct PluginRegistry<T> {
    entries: RwLock<Vec<PluginEntry<T>>>,
    default_order: RwLock<Vec<String>>,
}

impl<T> PluginRegistry<T> {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            default_order: RwLock::new(Vec::new()),
        }
    }

    pub fn register(&self, name: &str, factory: FactoryFn<T>) {
        self.entries.write().unwrap().push(PluginEntry {
            name: name.to_string(),
            factory,
            _lib: None,
        });
    }

    pub fn set_default_order(&self, order: Vec<String>) {
        *self.default_order.write().unwrap() = order;
    }

    pub fn create(&self, name: Option<&str>) -> Result<T, String> {
        let entries = self.entries.read().unwrap();
        let order = self.default_order.read().unwrap();

        if let Some(requested) = name {
            return entries
                .iter()
                .find(|e| e.name == requested)
                .map(|e| (e.factory)())
                .ok_or_else(|| format!("no backend named '{requested}'"));
        }

        let effective_order: Vec<&str> = if order.is_empty() {
            entries.iter().map(|e| e.name.as_str()).collect()
        } else {
            order.iter().map(|s| s.as_str()).collect()
        };

        let name_map: HashMap<&str, &PluginEntry<T>> =
            entries.iter().map(|e| (e.name.as_str(), e)).collect();

        for candidate in &effective_order {
            if let Some(entry) = name_map.get(candidate) {
                return Ok((entry.factory)());
            }
        }

        Err("no webrtc backend available".to_string())
    }
}

impl<T> Default for PluginRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_factory(value: &'static str) -> FactoryFn<String> {
        let s = value.to_string();
        Box::new(move || s.clone())
    }

    #[test]
    fn registry_returns_err_when_empty() {
        let registry = PluginRegistry::<String>::new();
        let result = registry.create(None);
        assert!(result.is_err());
    }

    #[test]
    fn registry_returns_ok_with_registered_backend() {
        let registry = PluginRegistry::new();
        registry.register("webrtc-rs", ok_factory("webrtc-rs-ok"));
        assert_eq!(registry.create(Some("webrtc-rs")).unwrap(), "webrtc-rs-ok");
    }

    #[test]
    fn registry_returns_err_for_unknown_name() {
        let registry = PluginRegistry::<String>::new();
        let result = registry.create(Some("nonexistent"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent"));
    }

    #[test]
    fn registry_fallback_to_next_when_first_not_registered() {
        let registry = PluginRegistry::new();
        registry.register("ok", ok_factory("ok-works"));
        registry.set_default_order(vec!["fail".into(), "ok".into()]);
        let result = registry.create(None);
        assert_eq!(result.unwrap(), "ok-works");
    }

    #[test]
    fn registry_uses_default_order_when_no_name_given() {
        let registry = PluginRegistry::new();
        registry.register("second", ok_factory("second-works"));
        registry.register("first", ok_factory("first-works"));
        registry.set_default_order(vec!["first".into(), "second".into()]);
        assert_eq!(registry.create(None).unwrap(), "first-works");
    }

    #[test]
    fn registry_returns_error_when_all_fail() {
        let registry = PluginRegistry::<String>::new();
        registry.set_default_order(vec!["fail1".into(), "fail2".into()]);
        let result = registry.create(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no webrtc backend"));
    }

    #[test]
    fn registry_exact_name_overrides_default_order() {
        let registry = PluginRegistry::new();
        registry.register("first", ok_factory("first"));
        registry.register("second", ok_factory("second"));
        registry.set_default_order(vec!["first".into(), "second".into()]);
        assert_eq!(registry.create(Some("second")).unwrap(), "second");
    }
}
