//! Big vector type, used with vectors that can't be serde'd

use {
    arrayref::array_ref,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        msg, program_error::ProgramError, program_memory::sol_memmove, program_pack::Pack,
    },
    std::marker::PhantomData,
};

/// Contains easy to use utilities for a big vector of Borsh-compatible types,
/// to avoid managing the entire struct on-chain and blow through stack limits.
pub struct BigVec<'data> {
    /// Underlying data buffer, pieces of which are serialized
    pub data: &'data mut [u8],
}

const VEC_SIZE_BYTES: usize = 4;

impl<'data> BigVec<'data> {
    /// Get the length of the vector
    pub fn len(&self) -> u32 {
        let vec_len = array_ref![self.data, 0, VEC_SIZE_BYTES];
        u32::from_le_bytes(*vec_len)
    }

    /// Find out if the vector has no contents (as demanded by clippy)
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Retain all elements that match the provided function, discard all others
    pub fn retain<T: Pack>(&mut self, predicate: fn(&[u8]) -> bool) -> Result<(), ProgramError> {
        let mut vec_len = self.len();
        let mut removals_found = 0;
        let mut dst_start_index = 0;

        let data_start_index = VEC_SIZE_BYTES;
        let data_end_index =
            data_start_index.saturating_add((vec_len as usize).saturating_mul(T::LEN));
        for start_index in (data_start_index..data_end_index).step_by(T::LEN) {
            let end_index = start_index + T::LEN;
            let slice = &self.data[start_index..end_index];
            if !predicate(slice) {
                let gap = removals_found * T::LEN;
                if removals_found > 0 {
                    // In case the compute budget is ever bumped up, allowing us
                    // to use this safe code instead:
                    // self.data.copy_within(dst_start_index + gap..start_index, dst_start_index);
                    unsafe {
                        sol_memmove(
                            self.data[dst_start_index..start_index - gap].as_mut_ptr(),
                            self.data[dst_start_index + gap..start_index].as_mut_ptr(),
                            start_index - gap - dst_start_index,
                        );
                    }
                }
                dst_start_index = start_index - gap;
                removals_found += 1;
                vec_len -= 1;
            }
        }

        // final memmove
        if removals_found > 0 {
            let gap = removals_found * T::LEN;
            // In case the compute budget is ever bumped up, allowing us
            // to use this safe code instead:
            //self.data.copy_within(dst_start_index + gap..data_end_index, dst_start_index);
            unsafe {
                sol_memmove(
                    self.data[dst_start_index..data_end_index - gap].as_mut_ptr(),
                    self.data[dst_start_index + gap..data_end_index].as_mut_ptr(),
                    data_end_index - gap - dst_start_index,
                );
            }
        }

        let mut vec_len_ref = &mut self.data[0..VEC_SIZE_BYTES];
        vec_len.serialize(&mut vec_len_ref)?;

        Ok(())
    }

    /// Extracts a slice of the data types
    pub fn deserialize_mut_slice<T: Pack>(
        &mut self,
        skip: usize,
        len: usize,
    ) -> Result<Vec<&'data mut T>, ProgramError> {
        let vec_len = self.len();
        if skip + len > vec_len as usize {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let start_index = VEC_SIZE_BYTES.saturating_add(skip.saturating_mul(T::LEN));
        let end_index = start_index.saturating_add(len.saturating_mul(T::LEN));
        let mut deserialized = vec![];
        for slice in self.data[start_index..end_index].chunks_exact_mut(T::LEN) {
            deserialized.push(unsafe { &mut *(slice.as_ptr() as *mut T) });
        }
        Ok(deserialized)
    }

    /// Add new element to the end
    /// Deprecated. Use `insert_in_order` instead
    pub fn push<T: Pack>(&mut self, element: T) -> Result<(), ProgramError> {
        let mut vec_len_ref = &mut self.data[0..VEC_SIZE_BYTES];
        let mut vec_len = u32::try_from_slice(vec_len_ref)?;

        let start_index = VEC_SIZE_BYTES + vec_len as usize * T::LEN;
        let end_index = start_index + T::LEN;

        vec_len += 1;
        vec_len.serialize(&mut vec_len_ref)?;

        if self.data.len() < end_index {
            return Err(ProgramError::AccountDataTooSmall);
        }
        let mut element_ref = &mut self.data[start_index..start_index + T::LEN];
        element.pack_into_slice(&mut element_ref);
        Ok(())
    }

    /// Get an iterator for the type provided
    pub fn iter<'vec, T: Pack>(&'vec self) -> Iter<'data, 'vec, T> {
        Iter {
            len: self.len() as usize,
            current: 0,
            current_index: VEC_SIZE_BYTES,
            inner: self,
            phantom: PhantomData,
        }
    }

    /// Get a mutable iterator for the type provided
    pub fn iter_mut<'vec, T: Pack>(&'vec mut self) -> IterMut<'data, 'vec, T> {
        IterMut {
            len: self.len() as usize,
            current: 0,
            current_index: VEC_SIZE_BYTES,
            inner: self,
            phantom: PhantomData,
        }
    }

    /// Find matching data in the array
    pub fn find<T: Pack + Ord>(&self, element: &T) -> Option<&T> {
        let (index, is_found) = self.binary_search(element);
        if is_found {
            Some(self.get::<T>(index).unwrap())
        } else {
            None
        }
    }

    /// Find matching data in the array
    pub fn find_mut<T: Pack + Ord>(&mut self, element: &T) -> Option<&mut T> {
        let (index, is_found) = self.binary_search(element);
        if is_found {
            Some(self.get_mut::<T>(index).unwrap())
        } else {
            None
        }
    }

    /// Returns either the index at which the element is found, and true
    /// or the index where the element should be, and false.

    /// The returned index is in the range [0, len] inclusive
    fn binary_search<T: Pack + Ord>(&self, element: &T) -> (usize, bool) {
        let len = self.len() as usize;
        if len == 0 {
            return (0, false);
        }
        let (mut min, mut max) = (0, len - 1);

        while min <= max {
            let mid = (max - min) / 2 + min;
            if let Some(elem_at_index) = self.get::<T>(mid) {
                if *elem_at_index == *element {
                    return (mid, true);
                } else if *elem_at_index < *element {
                    min = mid + 1;
                } else {
                    if mid == 0 {
                        return (0, false);
                    }
                    max = mid - 1;
                }
            } else {
                return (0, false);
            }
        }
        // If elem > elem_at_index, min = elem_at_index + 1
        // If min = elem_at_index > elem,
        return (min, false);
    }

    /// Add new element in order into an ordered vec
    pub fn insert_in_order<T: Pack + Ord + std::fmt::Debug>(
        &mut self,
        element: &T,
    ) -> Result<(), ProgramError> {
        let (index, is_found) = self.binary_search(element);
        if is_found {
            msg!(
                "Cannot insert existing element. Found existing at vec index {}",
                index
            );
            return Err(ProgramError::InvalidArgument);
        } else {
            let buffer_len = self.data.len();
            let mut vec_len_ref = &mut self.data[0..VEC_SIZE_BYTES];
            let mut vec_len = u32::try_from_slice(vec_len_ref)?;
            vec_len += 1;

            if (VEC_SIZE_BYTES + vec_len as usize * T::LEN) > buffer_len {
                return Err(ProgramError::AccountDataTooSmall);
            }
            vec_len.serialize(&mut vec_len_ref)?;

            let start = VEC_SIZE_BYTES + index * T::LEN;

            // vec_len * T::LEN = num_bytes_to_shift + (index + 1) * T::LEN
            // [...; (index + 1) * T::LEN] | [...; bytes_to_shift] = [..; vec_len]
            let bytes_to_shift = (vec_len as usize - 1 - index) * T::LEN;

            unsafe {
                sol_memmove(
                    self.data[start + T::LEN..].as_mut_ptr(),
                    self.data[start..].as_mut_ptr(),
                    bytes_to_shift,
                );
            }

            let mut element_ref = &mut self.data[start..start + T::LEN];
            element.pack_into_slice(&mut element_ref);

            Ok(())
        }
    }

    /// Find matching data in the array
    fn get<T: Pack>(&self, index: usize) -> Option<&T> {
        let len = self.len() as usize;
        if index < len {
            let start = VEC_SIZE_BYTES + index * T::LEN;
            let end = start + T::LEN;
            let slice = &self.data[start..end];
            return Some(unsafe { &*(slice.as_ptr() as *const T) });
        }
        None
    }

    /// Find matching data in the array
    fn get_mut<T: Pack>(&self, index: usize) -> Option<&mut T> {
        let len = self.len() as usize;
        if index < len {
            let start = VEC_SIZE_BYTES + index * T::LEN;
            let end = start + T::LEN;
            let slice = &self.data[start..end];
            return Some(unsafe { &mut *(slice.as_ptr() as *mut T) });
        }
        None
    }
}

/// Iterator wrapper over a BigVec
pub struct Iter<'data, 'vec, T> {
    len: usize,
    current: usize,
    current_index: usize,
    inner: &'vec BigVec<'data>,
    phantom: PhantomData<T>,
}

impl<'data, 'vec, T: Pack + 'data> Iterator for Iter<'data, 'vec, T> {
    type Item = &'data T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.len {
            None
        } else {
            let end_index = self.current_index + T::LEN;
            let value = Some(unsafe {
                &*(self.inner.data[self.current_index..end_index].as_ptr() as *const T)
            });
            self.current += 1;
            self.current_index = end_index;
            value
        }
    }
}

/// Iterator wrapper over a BigVec
pub struct IterMut<'data, 'vec, T> {
    len: usize,
    current: usize,
    current_index: usize,
    inner: &'vec mut BigVec<'data>,
    phantom: PhantomData<T>,
}

impl<'data, 'vec, T: Pack + 'data> Iterator for IterMut<'data, 'vec, T> {
    type Item = &'data mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.len {
            None
        } else {
            let end_index = self.current_index + T::LEN;
            let value = Some(unsafe {
                &mut *(self.inner.data[self.current_index..end_index].as_ptr() as *mut T)
            });
            self.current += 1;
            self.current_index = end_index;
            value
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, solana_program::program_pack::Sealed};

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct TestStruct {
        value: u64,
    }

    impl Sealed for TestStruct {}

    impl Pack for TestStruct {
        const LEN: usize = 8;
        fn pack_into_slice(&self, data: &mut [u8]) {
            let mut data = data;
            self.value.serialize(&mut data).unwrap();
        }
        fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
            Ok(TestStruct {
                value: u64::try_from_slice(src).unwrap(),
            })
        }
    }

    impl TestStruct {
        fn new(value: u64) -> Self {
            Self { value }
        }
    }

    fn from_slice<'data, 'other>(data: &'data mut [u8], vec: &'other [u64]) -> BigVec<'data> {
        let mut big_vec = BigVec { data };
        for element in vec {
            big_vec.push(TestStruct::new(*element)).unwrap();
        }
        big_vec
    }

    fn from_slice_in_order<'data, 'other>(
        data: &'data mut [u8],
        vec: &'other [u64],
    ) -> BigVec<'data> {
        let mut big_vec = BigVec { data };
        for element in vec {
            println!("{}", element);
            big_vec.insert_in_order(&TestStruct::new(*element)).unwrap();
        }
        big_vec
    }

    fn check_big_vec_eq(big_vec: &BigVec, slice: &[u64]) {
        assert!(big_vec
            .iter::<TestStruct>()
            .map(|x| &x.value)
            .zip(slice.iter())
            .all(|(a, b)| a == b));
    }

    #[test]
    fn push() {
        let mut data = [0u8; 4 + 8 * 3];
        let mut v = BigVec { data: &mut data };
        v.push(TestStruct::new(1)).unwrap();
        check_big_vec_eq(&v, &[1]);
        v.push(TestStruct::new(2)).unwrap();
        check_big_vec_eq(&v, &[1, 2]);
        v.push(TestStruct::new(3)).unwrap();
        check_big_vec_eq(&v, &[1, 2, 3]);
        assert_eq!(
            v.push(TestStruct::new(4)).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
    }

    #[test]
    fn retain() {
        fn mod_2_predicate(data: &[u8]) -> bool {
            u64::try_from_slice(data).unwrap() % 2 == 0
        }

        let mut data = [0u8; 4 + 8 * 4];
        let mut v = from_slice(&mut data, &[1, 2, 3, 4]);
        v.retain::<TestStruct>(mod_2_predicate).unwrap();
        check_big_vec_eq(&v, &[2, 4]);
    }

    #[test]
    fn check_in_order() {
        let mut data = [0u8; 4 + 8 * 4];
        let mut array = [6, 2, 8, 4];
        let v = from_slice_in_order(&mut data, &array);
        array.sort();

        for (i, item) in array.iter().enumerate() {
            println!("{}:{}", i, item);
            assert_eq!(*v.get::<TestStruct>(i).unwrap(), TestStruct::new(*item));
        }
    }

    #[test]
    fn find() {
        let mut data = [0u8; 4 + 8 * 4];
        let v = from_slice(&mut data, &[1, 2, 3, 4]);
        assert_eq!(
            v.find::<TestStruct>(&TestStruct::new(1u64)),
            Some(&TestStruct::new(1))
        );
        assert_eq!(
            v.find::<TestStruct>(&TestStruct::new(4u64)),
            Some(&TestStruct::new(4))
        );
        assert_eq!(v.find::<TestStruct>(&TestStruct::new(5u64)), None);
    }

    #[test]
    fn find_mut() {
        let mut data = [0u8; 4 + 8 * 4];
        let mut v = from_slice(&mut data, &[1, 2, 3, 4]);
        let mut test_struct = v.find_mut::<TestStruct>(&TestStruct::new(1u64)).unwrap();
        test_struct.value = 0;
        check_big_vec_eq(&v, &[0, 2, 3, 4]);
        assert_eq!(v.find_mut::<TestStruct>(&TestStruct::new(5u64)), None);
    }

    #[test]
    fn deserialize_mut_slice() {
        let mut data = [0u8; 4 + 8 * 4];
        let mut v = from_slice(&mut data, &[1, 2, 3, 4]);
        let mut slice = v.deserialize_mut_slice::<TestStruct>(1, 2).unwrap();
        slice[0].value = 10;
        slice[1].value = 11;
        check_big_vec_eq(&v, &[1, 10, 11, 4]);
        assert_eq!(
            v.deserialize_mut_slice::<TestStruct>(1, 4).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
        assert_eq!(
            v.deserialize_mut_slice::<TestStruct>(4, 1).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
    }
}
