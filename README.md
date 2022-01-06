# Shared Ptrs

For abstracting over different kind of threadsafe shared pointers with interior mutability but
with swappable interfaces.

This crate provides 3 implementations in order of complexity and performance impact:

* `rc_refcell::SharedPtr`: The single threaded shared pointer with runtime borrow checking
* `arc_mutex::SharedPtr`: The thread-safe shared pointer that provides interior mutability via
  parking_lot mutexes
* `arc_rwlock::SharedPtr`: The thread-safe shared pointer that provides interior mutability via
  a parking_lot rwlock

## Rationale

Some of the issues we've had with different types of concurrency
primitives is that they do not provide a similar api. Refcell uses `borrow()` and
`borrow_mut()`, Mutexes use `lock()`, and RwLock uses `read()` and `write()`. SharedPtr
abstracts over all of these with the help of type inference to provide just a `read()` and
`write()` interface, which reduces refactoring complexity when switching or experimenting with
different concurrency control

## Why not traits

That is much harder. See the archery crate.

## SharedPtr<dyn Potato> does not work

This is due to the CoercedUnsized feature which is not stable at all. When this is available it
will allow non stdlib implementations of smart pointers that directly store trait objects.

Until then, the workaround is to Box your trait object. `SharedPtr<Box<dyn Trait>>` which will
allow you to store the trait object at the cost of another level of indirection.

