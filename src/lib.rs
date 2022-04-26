/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#![forbid(unsafe_code)]

use std::cmp::min;
use std::io::{Error, ErrorKind, Read, Result, Write};

macro_rules! check_valid {
    ($self:ident) => {
        debug_assert!($self.read_pos < $self.data.len());
        debug_assert!($self.write_pos < $self.data.len());
    }
}

/// A fixed sized buffer connected end-to-end.
///
/// # Examples
///
/// ```
/// use std::io::{Read, Write};
///
/// use bytebufrs::RingBuf;
///
/// let mut rb = RingBuf::with_capacity(5);
///
/// rb.write(&[3, 2, 1]).unwrap();
/// assert_eq!(rb.len(), 3);
///
/// let mut buf = [0u8; 10];
/// rb.read(&mut buf).unwrap();
/// assert_eq!(rb.len(), 0);
/// assert_eq!(buf, [3, 2, 1, 0, 0, 0, 0, 0, 0, 0]);
///
/// rb.write(&[4, 5, 6, 7, 8]).unwrap();
/// assert_eq!(rb.len(), 5);
/// ```
pub struct RingBuf {
    data: Box<[u8]>,
    read_pos: usize,
    write_pos: usize
}

impl RingBuf {
    /// Constructs a new, empty `RingBuf` with the specified capacity.
    /// The underlying buffer will be of length `capacity + 1`;
    ///
    /// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let mut rb = RingBuf::with_capacity(10);
    ///
    /// assert_eq!(rb.capacity(), 10);
    /// assert_eq!(rb.len(), 0);
    /// assert!(rb.is_empty());
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        vec!(0; capacity + 1).into_boxed_slice().into()
    }

    /// Returns the number of bytes the ring buffer can hold.
    ///
    //// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let rb = RingBuf::with_capacity(10);
    /// assert_eq!(rb.capacity(), 10);
    /// ```
    pub fn capacity(&self) -> usize {
        check_valid!(self);

        self.data.len() - 1
    }

    /// Clears the ring buffer, resetting the read and write position to 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let mut rb: RingBuf = vec![0, 1, 2, 3, 4].into();
    ///
    /// rb.clear();
    ///
    /// assert_eq!(rb.len(), 0);
    /// assert!(rb.is_empty());
    /// ```
    pub fn clear(&mut self) {
        check_valid!(self);

        self.read_pos = 0;
        self.write_pos = 0;
    }

    /// Returns the number of bytes in the ring buffer, also referred to
    /// as its 'length'.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let rb: RingBuf = vec![1, 2, 3].into();
    /// assert_eq!(rb.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        check_valid!(self);

        if self.read_pos > self.write_pos {
            self.data.len() - self.read_pos + self.write_pos
        }
        else {
            self.write_pos - self.read_pos
        }
    }

    /// Returns `true` if the ring buffer doesn't contain any bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Write;
    ///
    /// use bytebufrs::RingBuf;
    ///
    /// let mut rb = RingBuf::with_capacity(10);
    /// assert!(rb.is_empty());
    ///
    /// rb.write(&[0, 1, 2, 3]).unwrap();
    /// assert!(!rb.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        check_valid!(self);

        self.read_pos == self.write_pos
    }

    /// Advances the read position by count.
    /// The read position can't go past the write position.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let mut rb: RingBuf = vec![0, 1, 2].into();
    /// assert_eq!(rb.len(), 3);
    /// rb.advance_read_pos(3).unwrap();
    /// assert_eq!(rb.len(), 0);
    /// ```
    ///
    /// Trying to set the read position past the write position will fail:
    ///
    /// ```should_panic
    /// use bytebufrs::RingBuf;
    ///
    /// let mut rb = RingBuf::with_capacity(10);
    /// rb.advance_read_pos(1).unwrap(); // Will panic!
    /// ```
    pub fn advance_read_pos(&mut self, count: usize) -> Result<()> {
        check_valid!(self);

        if count > self.len() {
            return Err(Error::new(ErrorKind::InvalidInput, "Can't seek past write pos."));
        }

        self.read_pos += count;
        if self.read_pos >= self.data.len() {
            self.read_pos -= self.data.len();
        }

        Ok(())
    }

    /// Reads from the ring buffer without advancing the read position.
    /// On success,returns the number of bytes peeked.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let mut rb: RingBuf = vec![0, 1, 2].into();
    /// assert_eq!(rb.len(), 3);
    ///
    /// let mut buf = [0u8; 10];
    /// rb.peek(&mut buf).unwrap();
    /// assert_eq!(buf, [0, 1, 2, 0, 0, 0, 0, 0, 0, 0]);
    /// assert_eq!(rb.len(), 3);
    /// ```
    pub fn peek(&self, buf: &mut [u8]) -> Result<usize> {
        check_valid!(self);

        let to_read = min(self.len(), buf.len());
        let bytes_until_end = self.data.len() - self.read_pos;
        if bytes_until_end <= to_read {
            buf[..bytes_until_end].copy_from_slice(&self.data[self.read_pos..]);
            buf[bytes_until_end..to_read].copy_from_slice(&self.data[..to_read - bytes_until_end]);
        }
        else {
            buf[..to_read].copy_from_slice(&self.data[self.read_pos..self.read_pos + to_read]);
        }

        Ok(to_read)
    }
}

impl From<Box<[u8]>> for RingBuf {
    /// Creates a ring buffer with the given slice as backing buffer.
    /// Note that the ring buffer capacity will be 1 byte less then the length of the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let rb: RingBuf = vec![0u8; 5].into_boxed_slice().into();
    /// assert_eq!(rb.capacity(), 4);
    /// ```
    fn from(s: Box<[u8]>) -> Self {
        RingBuf {
            data: s,
            read_pos: 0,
            write_pos: 0
        }
    }
}

impl From<Vec<u8>> for RingBuf {
    /// Creates a ring buffer from the given vector.
    /// The capacity and length of the ring buffer will be equal to the length of the vector
    /// i.e. the ring buffer will be full.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytebufrs::RingBuf;
    ///
    /// let rb: RingBuf = vec![0, 1, 2, 3, 4].into();
    /// assert_eq!(rb.capacity(), 5);
    /// assert_eq!(rb.len(), 5);
    /// assert!(!rb.is_empty());
    /// ```
    fn from(mut s: Vec<u8>) -> Self {
        s.push(0);
        let write_pos = s.len() - 1;
        RingBuf {
            data: s.into_boxed_slice(),
            read_pos: 0,
            write_pos
        }
    }
}

impl Read for RingBuf {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        check_valid!(self);

        let bytes_read = self.peek(buf)?;
        self.advance_read_pos(bytes_read)?;
        Ok(bytes_read)
    }
}

impl Write for RingBuf {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        check_valid!(self);

        let to_write = min(self.capacity() - self.len(), buf.len());
        let bytes_until_end = self.data.len() - self.write_pos;
        if bytes_until_end <= to_write {
            self.data[self.write_pos..].copy_from_slice(&buf[..bytes_until_end]);
            self.data[..to_write - bytes_until_end].copy_from_slice(&buf[bytes_until_end..to_write]);
            self.write_pos = to_write - bytes_until_end;
        }
        else {
            self.data[self.write_pos..self.write_pos + to_write].copy_from_slice(&buf[..to_write]);
            self.write_pos += to_write;
        }

        Ok(to_write)
    }

    fn flush(&mut self) -> Result<()> {
        check_valid!(self);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use crate::RingBuf;

    #[test]
    fn ringbuf_with_capacity() {
        let rb = RingBuf::with_capacity(4);

        assert_eq!(rb.capacity(), 4);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
    }

    #[test]
    fn ringbuf_from_vec() {
        let mut rb: RingBuf = vec![5, 4, 3, 2, 1].into();

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        let mut buf = [0u8; 10];
        assert_eq!(rb.peek(&mut buf).unwrap(), 5);
        assert_eq!(buf, [5, 4, 3, 2, 1, 0, 0, 0, 0, 0]);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        buf = [0u8; 10];
        assert_eq!(rb.read(&mut buf).unwrap(), 5);
        assert_eq!(buf, [5, 4, 3, 2, 1, 0, 0, 0, 0, 0]);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());

        buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_eq!(rb.write(&buf).unwrap(), 5);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        buf = [0u8; 10];
        assert_eq!(rb.read(&mut buf).unwrap(), 5);
        assert_eq!(buf, [0, 1, 2, 3, 4, 0, 0, 0, 0, 0]);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
    }

    #[test]
    fn ringbuf_wrapped_read_write() {
        let mut rb = RingBuf::with_capacity(5);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());

        let mut buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_eq!(rb.write(&mut buf).unwrap(), 5);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        let mut buf = [0u8; 3];

        assert_eq!(rb.peek(&mut buf).unwrap(), 3);
        assert_eq!(buf, [0, 1, 2]);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        buf = [0u8; 3];
        assert_eq!(rb.read(&mut buf).unwrap(), 3);
        assert_eq!(buf, [0, 1, 2]);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 2);
        assert!(!rb.is_empty());

        let mut buf = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        assert_eq!(rb.write(&mut buf).unwrap(), 3);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        buf = [0u8; 10];
        assert_eq!(rb.peek(&mut buf).unwrap(), 5);
        assert_eq!(buf, [3, 4, 9, 8, 7, 0, 0, 0, 0, 0]);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        buf = [0u8; 10];
        assert_eq!(rb.read(&mut buf).unwrap(), 5);
        assert_eq!(buf, [3, 4, 9, 8, 7, 0, 0, 0, 0, 0]);

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
    }

    #[test]
    fn ringbuf_clear() {
        let mut rb: RingBuf = vec![5, 4, 3, 2, 1].into();

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());

        rb.clear();

        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
    }

    #[test]
    fn ringbuf_peek_read_empty() {
        let mut rb = RingBuf::with_capacity(10);

        let mut buf = [0u8; 10];
        assert_eq!(rb.peek(&mut buf).unwrap(), 0);
        assert_eq!(rb.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn ringbuf_peek_read_0_len_buf() {
        let mut rb: RingBuf = vec![0, 1, 2].into();

        let mut buf = [0u8; 0];
        assert_eq!(rb.peek(&mut buf).unwrap(), 0);
        assert_eq!(rb.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn ringbuf_read_write_larger_then_capacity() {
        let mut rb = RingBuf::with_capacity(5);

        assert_eq!(rb.write(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap(), 5);

        let mut buf = [0u8; 10];
        assert_eq!(rb.read(&mut buf).unwrap(), 5);
        assert_eq!(buf, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0]);

        assert_eq!(rb.write(&[6, 7, 8, 9, 10, 11, 12, 13, 14, 15]).unwrap(), 5);
        assert_eq!(rb.read(&mut buf).unwrap(), 5);
        assert_eq!(buf, [6, 7, 8, 9, 10, 0, 0, 0, 0, 0]);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
    }

    #[test]
    fn ringbuf_read_write_buf_end() {
        let mut rb = RingBuf::with_capacity(5);

        assert_eq!(rb.write(&[1]).unwrap(), 1);

        let mut buf = [0u8; 10];
        assert_eq!(rb.read(&mut buf).unwrap(), 1);
        assert_eq!(buf, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert_eq!(rb.write(&[0, 1, 2, 3, 4]).unwrap(), 5);
        assert_eq!(rb.read(&mut buf).unwrap(), 5);
        assert_eq!(buf, [0, 1, 2, 3, 4, 0, 0, 0, 0, 0]);
        assert_eq!(rb.read_pos, 0);
        assert_eq!(rb.write_pos, 0);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
    }
}
