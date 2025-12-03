//! MethodInfo

use crate::flags::{
    METHOD_ATTRIBUTE_ABSTRACT, METHOD_ATTRIBUTE_FINAL, METHOD_ATTRIBUTE_MEMBER_ACCESS_MASK,
    METHOD_ATTRIBUTE_STATIC, METHOD_ATTRIBUTE_VIRTUAL,
};
use crate::{ExceptionRef, Il2CppClass, NonNullRef, Ref};
use il2cpp_sys_rs::{
    il2cpp_class_get_method_from_name, il2cpp_method_get_param_name, il2cpp_runtime_invoke, il2cpp_type_get_name,
    Il2CppObject, Il2CppType,
};
use std::ffi::{c_void, CStr};
use std::{fmt, ptr, slice};

/// MethodInfo handle
pub type MethodInfo = NonNullRef<il2cpp_sys_rs::MethodInfo, ()>;
/// Nullable MethodInfo handle
pub type MethodInfoRef = Ref<il2cpp_sys_rs::MethodInfo, ()>;

impl MethodInfo {
    /// Returns the method name
    #[inline]
    pub const fn name(self) -> &'static CStr {
        // Safety: `name` is never null
        unsafe { CStr::from_ptr(self.as_ref().name) }
    }

    /// Returns the parent class of the method
    ///
    /// # Panics
    ///
    /// Panics if the parent class pointer is null
    #[track_caller]
    #[inline]
    pub const fn declaring_type(self) -> Il2CppClass {
        Il2CppClass::from_ptr(self.as_ref().klass).unwrap()
    }

    /// Returns the method return type
    ///
    /// # Panics
    ///
    /// Panics if the return type pointer is null
    #[track_caller]
    #[inline]
    pub const fn return_type(self) -> NonNullRef<Il2CppType, ()> {
        Ref::new(self.as_ref().return_type as _).unwrap_non_null()
    }

    /// Returns the parameter count
    #[inline]
    pub const fn parameters_count(self) -> u8 {
        self.as_ref().parameters_count
    }

    /// Returns the parameters
    #[inline]
    pub const fn parameters<'a>(self) -> &'a [Ref<Il2CppType, ()>] {
        if self.as_ref().parameters.is_null() {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(self.as_ref().parameters as _, self.parameters_count() as _)
            }
        }
    }

    /// Parameter name at a specific index
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based parameter index
    ///
    /// # Panics
    ///
    /// Panics when `index > parameters_count`
    #[track_caller]
    #[inline]
    pub fn param_name(self, index: u8) -> &'static CStr {
        assert!(index < self.parameters_count(), "index > parameters_count");

        let name = unsafe { il2cpp_method_get_param_name(self.as_ptr(), index as u32) };
        if name.is_null() {
            c""
        } else {
            unsafe { CStr::from_ptr(name) }
        }
    }

    /// Returns the raw method flags
    #[inline]
    pub const fn flags(self) -> u32 {
        // todo: there is also `iflags` (ImplementationFlags), need to research the purpose of each
        self.as_ref().flags as u32
    }

    /// Field accessibility
    ///
    /// - 0x0000 - `[CompilerGenerated]`
    /// - 0x0001 - `private`
    /// - 0x0002 - `private protected`
    /// - 0x0003 - `internal`
    /// - 0x0004 - `protected`
    /// - 0x0005 - `protected internal`
    /// - 0x0006 - `public`
    #[inline]
    pub const fn accessibility(self) -> u32 {
        self.flags() & METHOD_ATTRIBUTE_MEMBER_ACCESS_MASK
    }

    /// Returns `true` the method is static
    #[inline]
    pub const fn is_static(self) -> bool {
        self.flags() & METHOD_ATTRIBUTE_STATIC != 0
    }

    /// Returns `true` the method is final
    #[inline]
    pub const fn is_final(self) -> bool {
        self.flags() & METHOD_ATTRIBUTE_FINAL != 0
    }

    /// Returns `true` the method is virtual
    #[inline]
    pub const fn is_virtual(self) -> bool {
        self.flags() & METHOD_ATTRIBUTE_VIRTUAL != 0
    }

    /// Returns `true` the method is abstract
    #[inline]
    pub const fn is_abstract(self) -> bool {
        self.flags() & METHOD_ATTRIBUTE_ABSTRACT != 0
    }

    /// Human-readable method signature
    ///
    /// # Warning
    ///
    /// The result is indicative only. It does not include details such as
    /// `ref`, `out`, or generic parameters.
    pub fn signature(self) -> String {
        let name = self.name().to_string_lossy();
        let ret = unsafe {
            let return_type = self.return_type();
            CStr::from_ptr(il2cpp_type_get_name(return_type.as_ptr())).to_string_lossy()
        };

        let mut params = Vec::new();
        for (i, ptype) in self.parameters().iter().enumerate() {
            let type_name =
                unsafe { CStr::from_ptr(il2cpp_type_get_name(ptype.as_ptr())).to_string_lossy() };
            let name = self.param_name(i as u8).to_string_lossy();

            params.push(format!("{type_name} {name}"));
        }

        format!("{ret} {name}({})", params.join(", "))
    }

    /// Invokes the method with arguments on a target object
    ///
    /// # Arguments
    ///
    /// * `object` - Target instance, null for a static method
    /// * `arguments` - Mutable slice of argument pointers
    #[inline]
    pub fn invoke<T>(
        self,
        object: Ref<T, ()>,
        arguments: &mut [*mut c_void],
    ) -> Result<Ref<Il2CppObject, ()>, ExceptionRef> {
        unsafe {
            let mut err = ptr::null_mut();
            let result = il2cpp_runtime_invoke(
                self.as_ptr(),
                object.as_ptr() as _,
                arguments.as_mut_ptr(),
                &mut err,
            );
            if err.is_null() {
                Ok(Ref::new(result))
            } else {
                Err(Ref::new(err))
            }
        }
    }
}

impl MethodInfo {
    /// Finds a method by name and parameter count
    ///
    /// # Arguments
    ///
    /// * `class` - Class containing the method
    /// * `name` - Simple method name
    ///   For generic **definitions**, include the arity suffix (e.g. `List`1`, `Dictionary`2`).
    ///   Do **not** include type arguments here. For nested types, use `Outer`1/Inner`2`.
    /// * `arity` - Number of parameters
    ///
    /// # Returns
    ///
    /// Method handle if found, otherwise `None`
    #[inline]
    pub(crate) fn from_name(class: Il2CppClass, name: &CStr, arity: i32) -> Option<Self> {
        unsafe {
            Ref::new(il2cpp_class_get_method_from_name(class.as_ptr(), name.as_ptr(), arity) as _)
                .non_null()
        }
    }
}

impl fmt::Display for MethodInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.signature(), f)
    }
}

impl fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.signature(), f)
    }
}
