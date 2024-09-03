# theus

Welding Rust to other languages. Theus is a procedural macro for seamlessly generating C-compatible functions from Rust structs and traits.

## Features

- Automatically generates C-compatible wrapper functions for Rust struct methods and trait implementations.
- Handles both regular `impl Struct` blocks and `impl Trait for Struct` blocks.
- Preserves doc comments from Rust functions in the generated C-compatible functions (useful for passing doc comments to tools like [cbindgen](https://github.com/mozilla/cbindgen/blob/master/docs.md) for later use).
- Enforces the use of `&mut self` instead of `&self` for all methods (never know what external code will do with your pointers)
- Wraps all non-scalar and non-pointer types in pointers for safe passing across the FFI boundary
- Allows direct passing of scalar types, without wrapping them in pointers

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
theus = "0.1.0"
```

Then, in your Rust code:

```rust
use theus::c_compatible;

struct MyStruct {
    value: i32,
}

#[c_compatible]
impl MyStruct {
    pub fn create(value: i32) -> Self {
        MyStruct { value }
    }

    pub fn get_value(&mut self) -> i32 {
        self.value
    }

    // Note the owned receiver. The struct
    // will be dropped by the borrow checker
    // once this function finishes execution
    pub fn destroy(self) {}
}

trait MyTrait {
    fn trait_method(&mut self, x: i32) -> i32;
}

#[c_compatible]
impl MyTrait for MyStruct {
    fn trait_method(&mut self, x: i32) -> i32 {
        self.value + x
    }
}
```

Theus will generate the following code at compile time. When you compile 
this to a `cdylib`, it can be used with any C-compatible interface.

```rust
#[no_mangle]
pub unsafe extern "C" fn mystruct_create() -> *mut MyStruct {
    Box::into_raw(Box::new(MyStruct::create()))
}

#[no_mangle]
pub extern "C" fn mystruct_get_value(ptr: *mut MyStruct) -> i32 {
    (unsafe { &mut *ptr }).get_value()
}

#[no_mangle]
pub extern "C" fn mystruct_destroy(ptr: *mut MyStruct) {
    unsafe { Box::from_raw(ptr) }.destroy()
}

#[no_mangle]
pub extern "C" fn mystruct_mytrait_trait_method(ptr: *mut MyStruct, x: i32) -> i32 {
    (unsafe { &mut *ptr }).trait_method(x)
}
```

## Namesake

[Gladys Theus (1923-2012)](https://en.wikipedia.org/wiki/File:Mis_Gladys_Theus,_one_of_the_fastest_and_most_efficient_welders_at_the_Kaiser_Company_Permanente_Metals_Corporation..._-_NARA_-_196355.jpg?useskin=vector)

> Miss Gladys Theus in 1945, **_one of the fastest and most efficient welders_** at the Kaiser Company Permanente Metals Corporation yards of Richmond, California. She is sticking to her job until final victory is won.
> 
> \- National Archives and Records Administration, Washington, US.