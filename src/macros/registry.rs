//! Macro registry for storage and lookup of macro definitions.
//!
//! # Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using the `err_msg!` or `err_ctx!` macro. See `src/diagnostics.rs` for macro arms and usage rules.
//!
//! Example:
//! ```rust
//! use sutra::err_msg;
//! let err = err_msg!(Validation, "Macro already registered");
//! assert!(matches!(err, sutra::SutraError::Validation { .. }));
//! ```
//!
//! All macro registration, lookup, and serialization errors use this system.
//!
//! # Macro Types
//! - **Function macros**: Native Rust functions (not serializable).
//! - **Template macros**: Serializable macro templates.
//!
//! # Features
//! - Register, lookup, and remove macros by name (case-sensitive; empty names are not recommended).
//! - Overwriting an existing macro is silent unless using `*_or_error` methods.
//! - Only template macros are serialized/deserialized; function macros are ignored during (de)serialization.
//!
//! # Thread Safety
//! This type is **not** thread-safe. To share between threads, wrap in a `std::sync::Mutex` or `RwLock`.
//!
//! # Serialization Example
//! ```rust
//! use sutra::macros::{MacroRegistry, MacroTemplate};
//! use sutra::ast::{WithSpan, Expr, Span};
//! use std::sync::Arc;
//! let mut reg = MacroRegistry::new();
//! let params = sutra::ast::ParamList { required: vec![], rest: None, span: Span::default() };
//! let body = Box::new(WithSpan { value: Arc::new(Expr::Number(0.0, Span::default())), span: Span::default() });
//! let template = MacroTemplate::new(params, body).unwrap();
//! reg.register_template("foo", template);
//! let json = serde_json::to_string(&reg).unwrap();
//! let reg2: MacroRegistry = serde_json::from_str(&json).unwrap();
//! assert!(reg2.contains("foo"));
//! ```
//!
//! # Summary Table
//! | Method                      | Overwrites | Error on Duplicate | Serializable | Notes                  |
//! |-----------------------------|------------|--------------------|--------------|------------------------|
//! | register                    | Yes        | No                 | N/A          | Function macro         |
//! | register_or_error           | No         | Yes                | N/A          | Function macro         |
//! | register_template           | Yes        | No                 | Yes          | Template macro         |
//! | register_template_or_error  | No         | Yes                | Yes          | Template macro         |
//! | unregister                  | N/A        | N/A                | N/A          | Removes by name        |
//! | lookup/contains             | N/A        | N/A                | N/A          | Case-sensitive lookup  |
//!
//! # See Also
//! - [`MacroFn`](crate::macros::types::MacroFn)
//! - [`MacroTemplate`](crate::macros::types::MacroTemplate)

use crate::macros::types::{MacroDef, MacroFn, MacroTemplate};
use crate::SutraError;
use crate::err_ctx;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// Macro registry for built-in and template macros.
///
/// Stores macro definitions by name. Names are case-sensitive and should not be empty.
/// Overwriting an existing macro is silent unless using `*_or_error` methods.
///
/// # Thread Safety
/// Not thread-safe. Use a mutex or similar if sharing between threads.
///
/// # Serialization
/// Only template macros are serialized. Attempting to serialize a registry with only function macros will result in an empty output.
///
/// # Example
/// ```rust
/// use sutra::macros::{MacroRegistry, MacroFn, MacroTemplate};
/// use sutra::ast::{WithSpan, Expr, Span};
/// use std::sync::Arc;
/// let mut reg = MacroRegistry::new();
/// let my_macro_fn: MacroFn = |node| Ok(node.clone());
/// reg.register("foo", my_macro_fn);
/// let params = sutra::ast::ParamList { required: vec![], rest: None, span: Span::default() };
/// let body = Box::new(WithSpan { value: Arc::new(Expr::Number(0.0, Span::default())), span: Span::default() });
/// let template = MacroTemplate::new(params, body).unwrap();
/// reg.register_template("bar", template);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MacroRegistry {
    /// Map from macro name to macro definition (built-in or template).
    pub macros: HashMap<String, MacroDef>,
}

impl MacroRegistry {
    /// Creates a new, empty macro registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sutra::macros::MacroRegistry;
    /// let registry = MacroRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new function macro with the given name.
    ///
    /// Names are case-sensitive and should not be empty.
    /// If a macro with this name already exists, it will be replaced.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `func` - The function that implements the macro
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// let old = reg.register("foo", my_macro_fn).unwrap();
    /// assert!(old.is_none());
    /// let old2 = reg.register("foo", my_macro_fn).unwrap();
    /// assert!(old2.is_some());
    /// ```
    pub fn register(&mut self, name: &str, func: MacroFn) -> Result<Option<MacroDef>, SutraError> {
        let old_macro = self.macros.insert(name.to_string(), MacroDef::Fn(func));
        Ok(old_macro)
    }

    /// Registers a new function macro, returning an error if it already exists.
    ///
    /// Names are case-sensitive and should not be empty.
    /// This is a safer alternative to `register` that prevents accidental overwrites of existing macros.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `func` - The function that implements the macro
    ///
    /// # Errors
    /// Returns an error if a macro with this name is already registered.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// reg.register_or_error("foo", my_macro_fn).unwrap();
    /// assert!(reg.register_or_error("foo", my_macro_fn).is_err());
    /// ```
    pub fn register_or_error(&mut self, name: &str, func: MacroFn) -> Result<(), SutraError> {
        if self.macros.contains_key(name) {
            return Err(err_ctx!(Validation, "Macro '{}' is already registered", name));
        }
        self.macros.insert(name.to_string(), MacroDef::Fn(func));
        Ok(())
    }

    /// Registers a template macro with the given name.
    ///
    /// Names are case-sensitive and should not be empty.
    /// If a macro with this name already exists, it will be replaced.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `template` - The template that defines the macro
    ///
    /// # Returns
    /// `Some(old_macro)` if a macro with this name was already registered, `None` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroTemplate};
    /// use sutra::ast::{WithSpan, Expr, Span};
    /// use std::sync::Arc;
    /// let mut reg = MacroRegistry::new();
    /// let params = sutra::ast::ParamList { required: vec![], rest: None, span: Span::default() };
    /// let body = Box::new(WithSpan { value: Arc::new(Expr::Number(0.0, Span::default())), span: Span::default() });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// let old = reg.register_template("foo", template.clone()).unwrap();
    /// assert!(old.is_none());
    /// let old2 = reg.register_template("foo", template).unwrap();
    /// assert!(old2.is_some());
    /// ```
    pub fn register_template(&mut self, name: &str, template: MacroTemplate) -> Result<Option<MacroDef>, SutraError> {
        let old_macro = self.macros
            .insert(name.to_string(), MacroDef::Template(template));
        Ok(old_macro)
    }

    /// Registers a template macro, returning an error if it already exists.
    ///
    /// Names are case-sensitive and should not be empty.
    /// This is a safer alternative to `register_template` that prevents accidental overwrites of existing macros.
    ///
    /// # Arguments
    /// * `name` - The name to register the macro under
    /// * `template` - The template that defines the macro
    ///
    /// # Errors
    /// Returns an error if a macro with this name is already registered.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroTemplate};
    /// use sutra::ast::{WithSpan, Expr, Span};
    /// use std::sync::Arc;
    /// let mut reg = MacroRegistry::new();
    /// let params = sutra::ast::ParamList { required: vec![], rest: None, span: Span::default() };
    /// let body = Box::new(WithSpan { value: Arc::new(Expr::Number(0.0, Span::default())), span: Span::default() });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// reg.register_template_or_error("foo", template.clone()).unwrap();
    /// assert!(reg.register_template_or_error("foo", template).is_err());
    /// ```
    pub fn register_template_or_error(
        &mut self,
        name: &str,
        template: MacroTemplate,
    ) -> Result<(), SutraError> {
        if self.macros.contains_key(name) {
            return Err(err_ctx!(Validation, "Macro '{}' is already registered", name));
        }
        self.macros
            .insert(name.to_string(), MacroDef::Template(template));
        Ok(())
    }

    /// Unregisters a macro by name.
    ///
    /// Names are case-sensitive.
    /// Returns `Some(macro)` if the macro was found and removed, `None` if it didn't exist.
    /// No effect if the macro does not exist.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// let removed = reg.unregister("foo");
    /// assert!(removed.is_some());
    /// assert!(!reg.contains("foo"));
    /// ```
    pub fn unregister(&mut self, name: &str) -> Option<MacroDef> {
        self.macros.remove(name)
    }

    /// Looks up a macro by name.
    ///
    /// Names are case-sensitive.
    /// Returns `Some(macro_def)` if found, `None` if not found.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// if let Some(_macro_def) = reg.lookup("foo") {
    ///     // Found macro
    /// }
    /// ```
    pub fn lookup(&self, name: &str) -> Option<&MacroDef> {
        self.macros.get(name)
    }

    /// Checks if a macro with the given name is registered.
    ///
    /// Names are case-sensitive.
    /// Returns `true` if a macro with this name exists, `false` otherwise.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// assert!(reg.contains("foo"));
    /// assert!(!reg.contains("nonexistent"));
    /// ```
    pub fn contains(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    /// Returns the number of registered macros.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// assert_eq!(reg.len(), 0);
    /// reg.register("foo", my_macro_fn);
    /// assert_eq!(reg.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.macros.len()
    }

    /// Returns true if the registry is empty.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::MacroRegistry;
    /// let reg = MacroRegistry::new();
    /// assert!(reg.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    /// Returns an iterator over macro names.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// reg.register("macro1", my_macro_fn);
    /// reg.register("macro2", my_macro_fn);
    /// let names: Vec<_> = reg.names().collect();
    /// assert_eq!(names.len(), 2);
    /// ```
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.macros.keys()
    }

    /// Returns an iterator over macro definitions.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// for (_name, _def) in reg.iter() {
    ///     // ...
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MacroDef)> {
        self.macros.iter()
    }

    /// Clears all registered macros.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroFn};
    /// let mut reg = MacroRegistry::new();
    /// let my_macro_fn: MacroFn = |node| Ok(node.clone());
    /// reg.register("foo", my_macro_fn);
    /// assert!(!reg.is_empty());
    /// reg.clear();
    /// assert!(reg.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.macros.clear();
    }
}

// ============================================================================
// SERIALIZATION SUPPORT
// ============================================================================

impl Serialize for MacroRegistry {
    /// Serializes the registry, including only template macros.
    ///
    /// Function macros are filtered out during serialization since function pointers cannot be serialized.
    /// Attempting to serialize a registry with only function macros will result in an empty output.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroTemplate};
    /// use sutra::ast::{WithSpan, Expr, Span};
    /// use std::sync::Arc;
    /// let mut reg = MacroRegistry::new();
    /// let params = sutra::ast::ParamList { required: vec![], rest: None, span: Span::default() };
    /// let body = Box::new(WithSpan { value: Arc::new(Expr::Number(0.0, Span::default())), span: Span::default() });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// reg.register_template("foo", template);
    /// let json = serde_json::to_string(&reg).unwrap();
    /// assert!(json.contains("foo"));
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Only serialize Template macros, skip Fn variants
        let template_macros: HashMap<String, &MacroTemplate> = self
            .macros
            .iter()
            .filter_map(|(name, def)| {
                if let MacroDef::Template(template) = def {
                    Some((name.clone(), template))
                } else {
                    None
                }
            })
            .collect();

        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("MacroRegistry", 1)?;
        s.serialize_field("macros", &template_macros)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for MacroRegistry {
    /// Deserializes the registry, creating template macros.
    ///
    /// Only template macros are deserialized, as function macros cannot be serialized/deserialized.
    ///
    /// # Example
    /// ```rust
    /// use sutra::macros::{MacroRegistry, MacroTemplate};
    /// use sutra::ast::{WithSpan, Expr, Span, ParamList};
    /// use std::sync::Arc;
    /// // Construct a MacroTemplate and serialize it to JSON
    /// let params = ParamList { required: vec![], rest: None, span: Span::default() };
    /// let body = Box::new(WithSpan { value: Arc::new(Expr::Number(0.0, Span::default())), span: Span::default() });
    /// let template = MacroTemplate::new(params, body).unwrap();
    /// let mut reg = MacroRegistry::new();
    /// reg.register_template("foo", template);
    /// let json = serde_json::to_string(&reg).unwrap();
    /// let reg2: MacroRegistry = serde_json::from_str(&json).unwrap();
    /// assert!(reg2.contains("foo"));
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct MacroRegistryHelper {
            macros: HashMap<String, MacroTemplate>,
        }

        let helper = MacroRegistryHelper::deserialize(deserializer)?;
        let macros = helper
            .macros
            .into_iter()
            .map(|(name, template)| (name, MacroDef::Template(template)))
            .collect();

        Ok(MacroRegistry { macros })
    }
}

// ============================================================================
// MACRO DEFINITION SERIALIZATION
// ============================================================================

impl Serialize for MacroDef {
    /// Serializes macro definitions.
    ///
    /// Only `Template` variants are serializable. Attempting to serialize a `Fn` variant will result in an error at runtime.
    ///
    /// # Example
    /// ```
    /// use sutra::macros::MacroDef;
    /// // let fn_macro = MacroDef::Fn(my_macro_fn);
    /// // serde_json::to_string(&fn_macro).unwrap(); // This will error
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MacroDef::Template(tmpl) => {
                serializer.serialize_newtype_variant("MacroDef", 0, "Template", tmpl)
            }
            MacroDef::Fn(_) => {
                // Native functions cannot be serialized - this should never be reached
                // when using the MacroRegistry serializer that filters them out
                Err(serde::ser::Error::custom(
                    "Cannot serialize MacroDef::Fn variant - use MacroRegistry serialization instead"
                ))
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum MacroDefHelper {
    Template(MacroTemplate),
}

impl<'de> Deserialize<'de> for MacroDef {
    /// Deserializes macro definitions.
    ///
    /// Only the `Template` variant is deserializable, as function pointers cannot be serialized/deserialized.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match MacroDefHelper::deserialize(deserializer)? {
            MacroDefHelper::Template(tmpl) => Ok(MacroDef::Template(tmpl)),
        }
    }
}
