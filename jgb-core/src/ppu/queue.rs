const CAPACITY: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArrayQueue<T> {
    arr: [T; CAPACITY],
    start: usize,
    len: usize,
}

impl<T> ArrayQueue<T> {
    pub(crate) const CAPACITY: usize = CAPACITY;
}

impl<T> ArrayQueue<T>
where
    T: Copy + Default,
{
    pub(crate) fn new() -> Self {
        Self {
            arr: [Default::default(); 16],
            start: 0,
            len: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn front(&self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        Some(self.arr[self.start])
    }

    pub(crate) fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let front = self.arr[self.start];
        self.start = (self.start + 1) % Self::CAPACITY;
        self.len -= 1;

        Some(front)
    }

    pub(crate) fn push_back(&mut self, value: T) {
        if self.len == Self::CAPACITY {
            panic!("ArrayQueue has exceeded capacity of {}", Self::CAPACITY,);
        }

        self.arr[(self.start + self.len) % Self::CAPACITY] = value;
        self.len += 1;
    }

    pub(crate) fn extend_back(&mut self, values: &[T]) {
        if values.is_empty() {
            return;
        }

        if self.len + values.len() > Self::CAPACITY {
            panic!(
                "exceeded capacity {}, cannot append slice of {} values to ArrayQueue of len {}",
                Self::CAPACITY,
                values.len(),
                self.len
            );
        }

        let start = self.start + self.len;
        let end = start + values.len();
        if end <= Self::CAPACITY {
            self.arr[start..end].copy_from_slice(values);
            self.len += values.len();
            return;
        }

        let first_len = Self::CAPACITY - start;
        self.arr[start..].copy_from_slice(&values[..first_len]);
        self.arr[..values.len() - first_len].copy_from_slice(&values[first_len..]);
        self.len += values.len();
    }
}

#[cfg(test)]
mod tests {
    use super::ArrayQueue;

    #[test]
    fn new() {
        let mut q: ArrayQueue<u8> = ArrayQueue::new();

        assert_eq!(None, q.front());
        assert_eq!(0, q.len());
        assert!(q.is_empty());
        assert_eq!(None, q.pop_front());
        assert_eq!(0, q.len());
        assert!(q.is_empty());
    }

    #[test]
    fn push_pop() {
        let mut q = ArrayQueue::new();

        q.push_back(55);
        assert_eq!(1, q.len());
        assert!(!q.is_empty());
        assert_eq!(Some(55), q.front());
        assert_eq!(Some(55), q.pop_front());
        assert_eq!(None, q.pop_front());
        assert_eq!(0, q.len());
        assert!(q.is_empty());
        assert_eq!(None, q.pop_front());
        assert_eq!(0, q.len());
        assert!(q.is_empty());
    }

    #[test]
    fn extend() {
        let mut q = ArrayQueue::new();

        q.extend_back(&[1, 3, 5, 7, 9, 11, 13, 15]);
        assert_eq!(8, q.len());
        assert_eq!(Some(1), q.front());
        assert_eq!(Some(1), q.pop_front());
        assert_eq!(7, q.len());
        assert_eq!(Some(3), q.front());

        q.extend_back(&[2, 4, 6, 8, 10, 12]);
        assert_eq!(13, q.len());
        assert_eq!(Some(3), q.front());

        assert_eq!(Some(3), q.pop_front());
        assert_eq!(Some(5), q.pop_front());
        assert_eq!(Some(7), q.pop_front());
        assert_eq!(10, q.len());
        assert_eq!(Some(9), q.front());

        q.extend_back(&[20, 40, 60, 80, 100, 120]);
        assert_eq!(16, q.len());
        assert_eq!(Some(9), q.front());

        for n in [9, 11, 13, 15, 2, 4, 6, 8, 10, 12, 20, 40, 60, 80, 100, 120] {
            assert_eq!(Some(n), q.pop_front());
        }

        assert_eq!(None, q.front());
        assert_eq!(None, q.pop_front());
        assert_eq!(0, q.len());
    }

    #[test]
    fn extend_to_capacity() {
        let mut q = ArrayQueue::new();

        q.extend_back(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        q.pop_front();
        q.pop_front();
        q.pop_front();

        q.extend_back(&[13, 14, 15, 16]);

        assert_eq!(13, q.len());

        for n in [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] {
            assert_eq!(Some(n), q.pop_front());
        }

        assert_eq!(None, q.front());
        assert_eq!(None, q.pop_front());
        assert_eq!(0, q.len());
    }

    #[test]
    #[should_panic(expected = "exceeded capacity")]
    fn capacity_exceeded() {
        let mut q = ArrayQueue::new();

        q.extend_back(&[0; 16]);
        q.push_back(0);
    }

    #[test]
    #[should_panic(expected = "exceeded capacity")]
    fn extend_capacity_exceeded() {
        let mut q = ArrayQueue::new();

        q.extend_back(&[0; 8]);
        q.extend_back(&[0; 9]);
    }
}
