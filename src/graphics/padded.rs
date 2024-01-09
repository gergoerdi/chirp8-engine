enum PaddedStep<T> {
    Before(usize, T),
    During(T),
    After(usize),
}

pub struct Padded<T> {
    padding: usize,
    step: PaddedStep<T>,
}

pub fn pad<T>(n: usize, iter: T) -> Padded<T> {
    Padded{ padding: n, step: PaddedStep::Before(n, iter) }
}

impl<T: Iterator> Iterator for Padded<T> {
    type Item = Option<T::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        match core::mem::replace(&mut self.step, PaddedStep::After(0)) {
            PaddedStep::Before(cnt, start) => {
                if cnt == 0 {
                    self.step = PaddedStep::During(start);
                    self.next()
                } else {
                    self.step = PaddedStep::Before(cnt - 1, start);
                    Some(None)
                }
            },
            PaddedStep::During(mut iter) => {
                match iter.next() {
                    Some(x) => {
                        self.step = PaddedStep::During(iter);
                        Some(Some(x))
                    },
                    None => {
                        self.step = PaddedStep::After(self.padding);
                        self.next()
                    }
                }
            },
            PaddedStep::After(cnt) => {
                if cnt == 0 {
                    None
                } else {
                    self.step = PaddedStep::After(cnt - 1);
                    Some(None)
                }
            },
        }
    }
}
