extern crate libc;
use libc::{c_char, c_float, c_int, c_uchar, c_void};
use std::ptr;
use std::slice::from_raw_parts;

use std::fmt::{Debug, Error, Formatter};

extern crate serde;
use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer};

/// Wrapper type for a pointer to an FSA object in `OpenFst`.
#[repr(C)]
pub struct fsa_t {
    t: c_char,
    fsa: *mut c_void,
}

/// An integerized `Arc` with logarithmic pobabilistic weight.
#[derive(PartialEq, Debug, Clone)]
#[repr(C)]
pub struct fsa_arc {
    pub from_state: c_int,
    pub to_state: c_int,
    pub label: c_int,
    pub weight: c_float,
}

/// Vector type for calls between C++, C and Rust.
#[repr(C)]
pub struct vec_t {
    inner_type: c_uchar,
    vec_obj: *mut c_void,
    first: *mut c_void,
    length: usize,
}

// c function wrappers for 'foreign/fsa.cpp'
#[link(name = "fsa")]
#[link(name = "fst")]
#[link(name = "stdc++")]
extern "C" {
    /// Encodes an FSA into a binary string.
    pub fn fsa_to_string(fsa: *const fsa_t) -> vec_t;
    /// Decodes an FSA from a binary string.
    pub fn fsa_from_string(binary: *const vec_t) -> fsa_t;

    /// Creates a new FSA from
    /// * the numer of states,
    /// * a list of accepting states, and
    /// * a list of arcs.
    pub fn fsa_from_arc_list(
        states: c_int,
        final_stats: *const vec_t,
        arc_list: *const vec_t,
    ) -> fsa_t;
    /// Returns the list of all arcs of an FSA.
    pub fn fsa_to_arc_list(fsa: *const fsa_t) -> vec_t;

    /// Returns the initial state of an FSA.
    pub fn fsa_initial_state(fsa: *const fsa_t) -> c_int;
    /// Returns the list of final states of an FSA.
    pub fn fsa_final_states(fsa: *const fsa_t) -> vec_t;

    /// Creates the n-best FSA that contains the n best runs of an FSA.
    pub fn fsa_n_best(fsa: *const fsa_t, n: c_int) -> fsa_t;
    /// Constructs the product of two FSA.
    pub fn fsa_intersect(a: *const fsa_t, b: *const fsa_t) -> fsa_t;
    /// Constructs the product of an FSA with the inverse of a second FSA.
    pub fn fsa_difference(a: *const fsa_t, b: *const fsa_t) -> fsa_t;

    /// Frees the object.
    pub fn fsa_free(fsa: *const fsa_t);
    /// Frees the object.
    pub fn vec_free(vec: *const vec_t);
}

impl Drop for fsa_t {
    fn drop(&mut self) {
        unsafe {
            fsa_free(self);
        };
    }
}

impl vec_t {
    /// Creates a new `vec_t` referencing to the slice owned by `vector`-
    pub fn new<T>(vector: &mut Vec<T>) -> Self {
        vector.shrink_to_fit();
        vec_t {
            inner_type: 255,
            length: vector.len(),
            first: vector.as_mut_ptr() as *mut c_void,
            vec_obj: ptr::null_mut(),
        }
    }

    /// Borrow the slice referenced by a `vec_t`-
    pub fn as_slice<T>(&self) -> &[T] {
        unsafe { from_raw_parts(self.first as *mut T, self.length) }
    }

    /// Clones all values in the slice referenced by a `vec_t`.
    pub fn to_vec<T>(&self) -> Vec<T>
    where
        T: Clone,
    {
        let slice = self.as_slice();
        slice.to_vec()
    }
}

impl Drop for vec_t {
    fn drop(&mut self) {
        if !ptr::eq(self.vec_obj, ptr::null_mut()) {
            unsafe {
                vec_free(self);
            }
        }
    }
}

impl Debug for fsa_t {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let cvec = unsafe { fsa_to_string(self) };
        let vr: Vec<c_char> = cvec.to_vec();
        write!(f, "{:?}", vr)
    }
}

impl Serialize for fsa_t {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let byte_string: Vec<c_char> = unsafe {
            let chars = fsa_to_string(self);
            chars.to_vec()
        };

        byte_string.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for fsa_t {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<fsa_t, D::Error> {
        let mut bytes = Vec::<c_char>::deserialize(deserializer)?;

        Ok(unsafe { fsa_from_string(&vec_t::new(&mut bytes)) })
    }
}

#[cfg(test)]
mod tests {
    use libc::{c_float, c_int};
    use super::*;

    #[test]
    fn simple_fsa() {
        let mut arcs = vec![
            fsa_arc {
                from_state: 0 as c_int,
                to_state: 0 as c_int,
                label: 1 as c_int,
                weight: 1.0 as c_float,
            },
        ];
        let mut finals = vec![0 as c_int];
        let arcs_: Vec<fsa_arc> = unsafe {
            let fsa =
                fsa_from_arc_list(1 as c_int, &vec_t::new(&mut finals), &vec_t::new(&mut arcs));
            let alist = fsa_to_arc_list(&fsa);
            let a_slice: &[fsa_arc] = alist.as_slice();
            a_slice.iter().map(|a| a.clone()).collect()
        };

        assert_eq!(arcs, arcs_);
    }
}
