//! # Shared Ptr Patterns
//!
//! For abstracting over different kind of threadsafe shared pointers with interior mutability but
//! with swappable interfaces.
//!
//! This crate provides 3 implementations in order of complexity and performance impact:
//!
//! * `rc_refcell::SharedPtr`: The single threaded shared pointer with runtime borrow checking
//! * `arc_mutex::SharedPtr`: The thread-safe shared pointer that provides interior mutability via
//!   parking_lot mutexes
//! * `arc_rwlock::SharedPtr`: The thread-safe shared pointer that provides interior mutability via
//!   a parking_lot rwlock
//!
//! ## Rationale
//!
//! Some of the issues we've had with different types of concurrency
//! primitives is that they do not provide a similar api. Refcell uses `borrow()` and
//! `borrow_mut()`, Mutexes use `lock()`, and RwLock uses `read()` and `write()`. SharedPtr
//! abstracts over all of these with the help of type inference to provide just a `read()` and
//! `write()` interface, which reduces refactoring complexity when switching or experimenting with
//! different concurrency control
//!
//! ## Why not traits
//!
//! That is much harder. See the archery crate.
//!
//! ## SharedPtr<dyn Potato> does not work
//!
//! This is due to the CoercedUnsized feature which is not stable at all. When this is available it
//! will allow non stdlib implementations of smart pointers that directly store trait objects.
//!
//! Until then, the workaround is to Box your trait object. `SharedPtr<Box<dyn Trait>>` which will
//! allow you to store the trait object at the cost of another level of indirection.
#![allow(clippy::new_without_default)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(missing_debug_implementations)]
#![deny(warnings)]
mod deps {
    pub use ::derive_more;
    pub use ::owning_ref;
    pub use ::parking_lot;
    #[cfg(feature = "serde")]
    pub use ::serde;
}

macro_rules! define_shared_mut {
    ($name:ident, $weak_name:ident, $ptr:ident, $weak_ptr:ident, $guard:ident, $read_fn:ident, $write_fn:ident, $read_guard:ident, $write_guard:ident) => {
        #[derive(derive_more::From)]
        pub struct $name<T: ?Sized>($ptr<$guard<T>>);


        impl<T: Sized> $name<T> {
            pub fn new(init: T) -> Self {
                $name($ptr::new($guard::new(init)))
            }
        }

        impl<T: ?Sized> $name<T> {
            pub fn read(&self) -> $read_guard<'_, T> {
                self.0.deref().$read_fn()
            }

            pub fn write(&self) -> $write_guard<'_, T> {
                self.0.deref().$write_fn()
            }
        }

        // TODO(dillybar): do we still need this?
        #[allow(dead_code)]
        impl<T> $name<T> {
            pub(crate) fn as_ptr(&self) -> *const T {
                <$ptr<$guard<T>>>::as_ptr(&self.0) as *const T
            }
        }

        impl<T> std::fmt::Debug for $name<T>
        where
            T: std::fmt::Debug,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.debug_tuple("$name").field(&self.read()).finish()
            }
        }

        #[cfg(feature = "serde")]
        impl<'de, T> crate::deps::serde::de::Deserialize<'de> for $name<T>
        where
            T: Sized + crate::deps::serde::de::Deserialize<'de>,
        {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: crate::deps::serde::Deserializer<'de>,
            {
                Ok($name::new(T::deserialize(deserializer)?))
            }
        }

        #[cfg(feature = "serde")]
        impl<T> crate::deps::serde::ser::Serialize for $name<T>
        where
            T: crate::deps::serde::ser::Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: crate::deps::serde::Serializer,
            {
                let inner: $read_guard<'_, T> = self.0.deref().$read_fn();
                inner.serialize(serializer)
            }
        }

        impl<T: ?Sized> Clone for $name<T> {
            fn clone(&self) -> Self {
                $name(self.0.clone())
            }
        }

        impl<T> std::cmp::PartialEq for $name<T>
        where
            T: Sized + PartialEq,
        {
            fn eq(&self, other: &Self) -> bool {
                self.read().eq(&other.read())
            }
        }

        impl<T> std::cmp::Eq for $name<T> where T: Sized + Eq {}

        impl<T> std::cmp::PartialOrd for $name<T>
        where
            T: Sized + PartialOrd,
        {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.read().partial_cmp(&other.read())
            }
        }

        impl<T> std::cmp::Ord for $name<T>
        where
            T: Sized + Ord,
        {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.read().cmp(&other.read())
            }
        }

        impl<T> std::hash::Hash for $name<T>
        where
            T: Sized + std::hash::Hash,
        {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.read().hash(state)
            }
        }

        impl<T> Default for $name<T>
        where
            T: Sized + Default,
        {
            fn default() -> Self {
                $name::new(T::default())
            }
        }

        #[derive(Debug)]
        pub struct $weak_name<T: ?Sized>($weak_ptr<$guard<T>>);


        impl<T: ?Sized> $weak_name<T> {
            pub fn downgrade(strong: &$name<T>) -> Self {
                $weak_name($ptr::downgrade(&strong.0))
            }

            pub fn upgrade(&self) -> Option<$name<T>> {
                self.0.upgrade().map($name)
            }
        }


        impl<T> $weak_name<T>
        where
            T: Sized,
        {
            pub fn new() -> $weak_name<T> {
                $weak_name(<$weak_ptr<$guard<T>>>::new())
            }
        }

        #[test]
        fn test_interior_mutability() {
            use std::collections::HashMap;
            let mut map = HashMap::<usize, $name<u32>>::new();

            let answer = $name::new(0u32);
            for i in 1..=1024usize {
                assert!(map.insert(i, answer.clone()).is_none());
            }

            *(answer.write()) = 42u32;
            assert!(map.values().all(|v| *(v.read()) == 42u32))
        }
    };
}

pub mod rc_refcell {
    use core::cell::{
        Ref,
        RefCell,
        RefMut,
    };
    use std::rc::Weak;
    use std::{
        ops::Deref,
        rc::Rc,
    };

    use crate::deps::owning_ref::RefRef;

    pub type FieldRef<'a, T, V> = RefRef<'a, T, V>;

    define_shared_mut!(SharedPtr, WeakPtr, Rc, Weak, RefCell, borrow, borrow_mut, Ref, RefMut);
}

pub mod arc_mutex {
    use crate::deps::owning_ref::OwningRef;

    use std::ops::Deref;
    use std::sync::{
        Arc,
        Weak,
    };

    use crate::deps::parking_lot::{
        Mutex,
        MutexGuard,
    };

    pub type FieldRef<'a, T, V> = OwningRef<MutexGuard<'a, T>, V>;

    define_shared_mut!(SharedPtr, WeakPtr, Arc, Weak, Mutex, lock, lock, MutexGuard, MutexGuard);
}

pub mod arc_rwlock {
    use crate::deps::owning_ref::OwningRef;

    use std::ops::Deref;
    use std::sync::{
        Arc,
        Weak,
    };

    use crate::deps::parking_lot::{
        RwLock,
        RwLockReadGuard,
        RwLockWriteGuard,
    };

    pub type FieldRef<'a, T, V> = OwningRef<RwLockReadGuard<'a, T>, V>;

    define_shared_mut!(
        SharedPtr,
        WeakPtr,
        Arc,
        Weak,
        RwLock,
        read,
        write,
        RwLockReadGuard,
        RwLockWriteGuard
    );
}
