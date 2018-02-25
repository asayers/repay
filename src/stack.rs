use std::rc::Rc;

#[derive(Debug)]
pub enum Stack<T> {
    Nil,
    Cons(Rc<(usize, T, Stack<T>)>),
}

impl<T> Clone for Stack<T> {
    fn clone(&self) -> Self {
        match self {
            &Stack::Nil => Stack::Nil,
            &Stack::Cons(ref rc) => Stack::Cons(rc.clone()),
        }
    }
}

impl<T> Stack<T> {
    pub fn new() -> Stack<T> { Stack::Nil }
    pub fn push(&self, x: T) -> Stack<T> { Stack::Cons(Rc::new((self.len() + 1, x, self.clone()))) }
    pub fn len(&self) -> usize { match self { &Stack::Nil => 0, &Stack::Cons(ref rc) => rc.0 } }
    pub fn iter(&self) -> Iter<T> { Iter(self.clone()) }
}

pub struct Iter<T>(Stack<T>);
impl<T> Iterator for Iter<T> {
    type Item = Rc<(usize, T, Stack<T>)>;
    fn next(&mut self) -> Option<Self::Item> {
        let x = self.0.clone();
        match x {
            Stack::Nil => None,
            Stack::Cons(rc) => {
                self.0 = rc.2.clone();
                Some(rc)
            }
        }
    }
}
