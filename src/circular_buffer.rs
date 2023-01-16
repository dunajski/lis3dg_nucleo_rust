pub struct CircularBuff<T, const N: usize> {
    pub buf: [T; N],
    pub ri: usize,
    pub wi: usize,
}

impl<T: Copy, const N: usize> CircularBuff<T, N> {
    pub fn put_data(&mut self, data: T) {
        self.buf[self.wi] = data;

        if self.wi == self.buf.len() - 1 {
            self.wi = 0;
        } else {
            self.wi += 1;
        }
    }

    pub fn put_all_data(&mut self, data: &[T]) {
        for d in data {
            self.put_data(*d);
        }
    }

    pub fn get_data(&mut self) -> Option<T> {
        let mut data = None;
        if self.wi != self.ri {
            data = Some(self.buf[self.ri]);

            if self.ri == self.buf.len() - 1 {
                self.ri = 0;
            } else {
                self.ri += 1;
            }
        }
        data
    }

    // use this function to get all data in loop
    // while get_all_data != None
    pub fn get_all_data(&mut self) -> Option<T> {
        self.get_data()
    }
}
