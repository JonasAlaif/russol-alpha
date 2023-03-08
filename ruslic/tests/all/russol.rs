#![feature(box_patterns)]
use russol_contracts::*;

/* ########## */
/* ##### Benchmarks from examples in the paper ##### */
/* ########## */

// Version of SLL in paper
enum Node<T> { Nil, Cons { elem: T, next: Box<Node<T>> } }
impl<T> Node<T> {
    #[pure]
    fn len(&self) -> u16 {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }

    pub fn singleton(elem: T) -> Self {
      let next = Box::new(Node::Nil);
      Node::Cons { elem, next }
    }

    #[requires(self.len() > 0)]
    pub fn peek(&self) -> &T {
      match self {
        Node::Nil => unreachable!(),
        Node::Cons { elem, .. } => elem,
      }
    }

    #[ensures((^self).len() == self.len() + 1)]
    pub fn push_len(&mut self, elem: T) {
      match self {
        Node::Nil => {
          let next = Box::new(Node::Nil);
          let new = Node::Cons { elem, next };
          *self = new
        }
        Node::Cons { next, .. } => next.push_len(elem),
      }
    }

    #[ensures(match ^self {
        Node::Cons { ref next, .. } => **next === *self,
        Node::Nil => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = replace(self, Node::Nil);
      let next = Box::new(result);
      let new = Node::Cons { elem, next };
      *self = new
    }

    #[ensures(self.len() > 0 ==>
        (^self).len() == self.len()-1 && is_some(&result)
    )]
    pub fn pop(&mut self) -> Option<T> {
      let result = replace(self, Node::Nil);
      match result {
        Node::Nil => None,
        Node::Cons { elem, next } => {
          *self = *next;
          Some(elem)
        }
      }
    }

    #[ensures((^self).len() == self.len() + (^result).len())]
    pub fn peek_last(&mut self) -> &mut Self {
      match self {
        Node::Nil => self,
        Node::Cons { next, .. } => next.peek_last(),
      }
    }
}

// More efficient version of an SLL
enum NodeAlt<T> { Nil, Cons { node: Box<(T, NodeAlt<T>)> } }
impl<T> NodeAlt<T> {
    #[pure]
    fn len(&self) -> u16 {
        match self {
            NodeAlt::Nil => 0,
            NodeAlt::Cons { node } => 1 + node.1.len(),
        }
    }

    pub fn singleton(elem: T) -> Self {
      let bx = (elem, NodeAlt::Nil);
      let node = Box::new(bx);
      NodeAlt::Cons { node }
    }

    #[requires(self.len() > 0)]
    pub fn peek(&self) -> &T {
      match self {
        NodeAlt::Nil => unreachable!(),
        NodeAlt::Cons { node } => &node.0,
      }
    }

    #[ensures((^self).len() == self.len() + 1)]
    pub fn push_len(&mut self, elem: T) {
      match self {
        NodeAlt::Nil => {
          let bx = (elem, NodeAlt::Nil);
          let node = Box::new(bx);
          let new = NodeAlt::Cons { node };
          *self = new
        }
        NodeAlt::Cons { node } => node.1.push_len(elem),
      }
    }

    #[ensures(match ^self {
        NodeAlt::Cons { ref node } => node.1 === *self,
        NodeAlt::Nil => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = replace(self, NodeAlt::Nil);
      let bx = (elem, result);
      let node = Box::new(bx);
      let new = NodeAlt::Cons { node };
      *self = new
    }

    #[ensures(self.len() > 0 ==>
        (^self).len() == self.len()-1 && is_some(&result)
    )]
    pub fn pop(&mut self) -> Option<T> {
      let result = replace(self, NodeAlt::Nil);
      match result {
        NodeAlt::Nil => None,
        NodeAlt::Cons { node } => {
          *self = node.1;
          Some(node.0)
        }
      }
    }

    #[ensures((^self).len() == self.len() + (^result).len())]
    pub fn peek_last(&mut self) -> &mut Self {
      match self {
        NodeAlt::Nil => self,
        NodeAlt::Cons { node } => node.1.peek_last(),
      }
    }
}

/* ########## */
/* ##### Custom benchmarks ##### */
/* ########## */

#[requires(***x == 7_i32)]
#[ensures(  ^ ^^x == ^^y)]
#[ensures(  * ^^x == *^y)]
#[ensures(  ^ *^x == ^z)]
#[ensures(  **^x == 11_i32)]
fn triple_nested<'a, 'b>(x: &mut &'b mut &'a mut i32, y: &'b mut &'a mut i32, z: &'a mut i32) {
  *z = 11 as i32;
  *y = z;
  *x = y
}

#[ensures(  ^^y == ^z)]
fn triple_nested_2<'a, 'b>(x: &mut &'b mut &'a mut i32, y: &'b mut &'a mut i32, z: &'a mut i32) {
  *y = z
}

#[ensures(*^x == ^result)]
fn borrow_in<'a>(x: &'a mut &mut i32) -> &'a mut i32 {
  &mut **x
}

#[requires(**x + *y <= u16::MAX)]
#[ensures(*^x == **x + *y)]
#[ensures(^^x == ^y)]
fn write_and_reborrow<'a>(x: &mut &'a mut u16, y: &'a mut u16) {
  let de = *y;
  let de_de = **x;
  *y = (de_de + de) as u16;
  *x = y
}

#[requires(**x <= 100_u8)]
#[requires(^result <= 100_u8)]
#[ensures(*^x <= 100_u8)]
#[ensures(*result <= 100_u8)]
fn pledge(x: &mut Box<u8>) -> &mut u8 {
  &mut **x
}

mod private {
    use russol_contracts::*;
    pub struct Percentage(u16);

    #[extern_spec]
    #[ensures(*result == p.0)]
    #[ensures(^result == (^p).0)]
    #[ensures(if ^result == 10 { is_ten(&^p) } else { true })]
    pub fn bar(p: &mut Percentage) -> &mut u16 {
        &mut p.0
    }

    #[pure] pub fn is_ten(p: &Percentage) -> bool { p.0 == 10 }
}

#[ensures(private::is_ten(&^p))]
fn percetage_inner(p: &mut private::Percentage) {
  let result = private::bar(p);
  *result = 10 as u16
}

fn clone_check_1<T: Clone>(x: &T) -> T {
  x.clone()
}
struct Foo<T>(T);
impl<T: Clone> Foo<T> {
    fn clone_check_2(&self) -> Self {
      let result = self.0.clone();
      Foo(result)
    }
}

struct BorrowAndValue<'a, T> { borrow: &'a mut T, value: T }
impl<'a, T> BorrowAndValue<'a, T> {
    fn new(borrow: &'a mut T, b2: &mut i32) -> Self where T: Copy {
      let de = *borrow;
      BorrowAndValue { borrow, value: de }
    }
}

#[derive(Copy, Clone)]
struct Foo2(i32);
fn copy_out(x: &Foo2) -> Foo2 {
  *x
}

impl<T> Node<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Node::Nil => set!{},
            Node::Cons { elem, next } => next.elems() + set!{ elem },
        }
    }

    #[ensures(result.len() == self.len())]
    fn modify_elems(&mut self) -> Node<&mut T> {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let result = next.modify_elems();
          let next = Box::new(result);
          Node::Cons { elem, next }
        }
      }
    }
}
struct Tuple<'b, 'c>(&'b mut i32, &'c mut Node<i32>);
impl Node<i32> {
    #[pure]
    fn sum(&self) -> i32 {
        match self {
            Node::Nil => 0,
            Node::Cons { elem, next } => *elem + next.sum(),
        }
    }
    #[pure]
    fn elems_eq(&self) -> bool {
        match self {
            Node::Cons { elem, next: box next@Node::Cons { elem: fnxt, .. }, .. } =>
                *elem == *fnxt && next.elems_eq(),
            _ => true,
        }
    }
    #[pure]
    #[trusted_ensures(result >= 0)]
    fn len_gt0(&self) -> u16 {
        match self {
            Node::Nil => 0,
            Node::Cons { next, .. } => 1 + next.len(),
        }
    }

    #[ensures(result.len_gt0() == self.len_gt0())]
    #[ensures(if result.len_gt0() == 0 { result.elems() == set!{} } else { result.elems() == set!{&0} })]
    #[ensures(^self === ^result)]
    fn zero(&mut self) -> &mut Self {
      Self::zero_4(self);
      self
    }
    #[helper] fn zero_4(&mut self) {
      match self {
        Node::Nil => (),
        Node::Cons { elem, next } => {
          next.zero_4();
          *elem = 0 as i32
        }
      }
    }
    
    #[requires(self.len_gt0() <= u16::MAX)]
    #[ensures((^self).len_gt0() == self.len_gt0())]
    #[ensures(result == self.len_gt0())]
    fn calc_len(&mut self) -> u16 {
      match self {
        Node::Nil => 0 as u16,
        Node::Cons { next, .. } => {
          let result = next.calc_len();
          (result + 1) as u16
        }
      }
    }

    #[requires(i.len() >= 2)]
    #[ensures((^i).len() == 2 + (^result.1).len())]
    #[ensures((^i).elems() == match &i {
        Node::Cons { elem, .. } => Set::new(&[elem, &^result.0]) + (^result.1).elems(),
        _ => set!{},
    })]
    fn reborrow_head_and_tail_2<'a: 'b + 'c, 'b, 'c>(i: &'a mut &mut Self) -> Tuple<'b, 'c> {
        match &mut **i {
            Node::Nil => unreachable!(),
            Node::Cons { next, .. } => match &mut **next {
                Node::Nil => unreachable!(),
                Node::Cons { elem, next } => Tuple(elem, &mut **next),
            },
        }
    }

    #[requires(self.len() >= 2)]
    #[ensures(self.len() - 2 == result.len())]
    fn tail2(self) -> Self {
        match self {
            Node::Nil => unreachable!(),
            Node::Cons { next, .. } => match *next {
                Node::Nil => unreachable!(),
                Node::Cons { next, .. } => *next,
            },
        }
    }
}

struct List { head: Node<i32> }
impl List {
    #[ensures((^self).head.len() == (*self).head.len() + tl.head.len())]
    #[ensures((^self).head.elems() == (*self).head.elems() + tl.head.elems())]
    fn append(&mut self, tl: Self) {
        Self::append_8(&mut self.head, tl.head)
    }
    #[helper] fn append_8(head_self: &mut Node<i32>, head_tl: Node<i32>) {
        match head_self {
            Node::Nil => *head_self = head_tl,
            Node::Cons { next, .. } => Self::append_8(&mut **next, head_tl),
        }
    }

    #[ensures((*self).head.len() == (^self).head.len())]
    #[ensures((^self).head.elems_eq() == true)]
    fn lstset(&mut self) {
      let new = Self::lstset_7(&mut self.head);
      self.head = new
    }
    #[helper] fn lstset_7(head: &mut Node<i32>) -> Node<i32> {
      match head {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let new = Self::lstset_7(&mut **next);
          match new {
            Node::Nil => {
              let next = Box::new(Node::Nil);
              Node::Cons { elem: de as i32, next }
            }
            Node::Cons { elem, next } => {
              let bx = Node::Cons { elem: elem as i32, next };
              let next = Box::new(bx);
              Node::Cons { elem: elem as i32, next }
            }
          }
        }
      }
    }

    #[ensures(self.head.len() == result.head.len())]
    #[ensures(self.head.elems() == result.head.elems())]
    fn duplicate(&self) -> Self {
        Self::new_list_6(&self.head)
    }
    #[helper] fn new_list_6(head: &Node<i32>) -> Self {
        match head {
            Node::Nil => List { head: Node::Nil },
            Node::Cons { elem, next } => {
                let de = *elem;
                let result = Self::new_list_6(&**next);
                let next = Box::new(result.head);
                let head = Node::Cons { elem: de as i32, next };
                List { head }
            }
        }
    }

    #[ensures((^self).head.len() == self.head.len())]
    #[ensures((^self).head.sum() == 0)]
    fn zero(&mut self) {
      Self::zero_6(&mut self.head)
    }
    #[helper] fn zero_6(head: &mut Node<i32>) {
      match head {
        Node::Nil => (),
        Node::Cons { elem, next } => {
          Self::zero_6(&mut **next);
          *elem = 0 as i32
        }
      }
    }
}

enum Tree<T> { Leaf, Node { f: T, left: Box<Tree<T>>, right: Box<Tree<T>>, } }
impl<T> Tree<T> {
    #[pure]
    fn size(&self) -> u16 {
        match self {
            Tree::Leaf => 0,
            Tree::Node { left, right, .. } => 1 + left.size() + right.size(),
        }
    }
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            Tree::Leaf => set!{},
            Tree::Node { f, left, right } => left.elems() + right.elems() + set!{f},
        }
    }

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    #[params("--memo=false")]
    fn to_list(self) -> Node<T> {
      match self {
        Tree::Leaf => Node::Nil,
        Tree::Node { f, left, right } => {
          let result = left.to_list();
          Self::to_list_12(f, *right, result)
        }
      }
    }
    #[helper] fn to_list_12(f: T, bx: Tree<T>, result: Node<T>) -> Node<T> {
      match result {
        Node::Nil => {
          let result = bx.to_list();
          let next = Box::new(result);
          Node::Cons { elem: f, next }
        }
        Node::Cons { elem, next } => {
          let result = Self::to_list_12(elem, bx, *next);
          let next = Box::new(result);
          Node::Cons { elem: f, next }
        }
      }
    }
}
impl Tree<i32> {
    #[ensures(result.size() == self.size())]
    fn duplicate(&self) -> Self {
      match self {
        Tree::Leaf => Tree::Leaf,
        Tree::Node { f, left, right } => {
          let de = *f;
          let result_1 = left.duplicate();
          let result_2 = right.duplicate();
          let right = Box::new(result_2);
          let left = Box::new(result_1);
          Tree::Node { f: de as i32, left, right }
        }
      }
    }

    #[pure]
    fn ordered(&self) -> bool {
        match self {
            Tree::Leaf => true,
            Tree::Node { f, left, right } => {
                let lord = if let box Tree::Node { f: f_l, .. } = left { *f_l <= *f } else { true };
                let rord = if let box Tree::Node { f: f_r, .. } = right { *f <= *f_r } else { true };
                lord && rord && left.ordered() && right.ordered()
            },
        }
    }

    #[ensures(match (&self, &^self) {
        (Tree::Leaf, _) => (^self).elems() == set!{&v},
        (Tree::Node { f, left, right },
         Tree::Node { left: l, right: r, .. }) => {
            if v < *f {
                l.elems() == set!{&v} + left.elems()
            } else if *f < v {
                r.elems() == set!{&v} + right.elems()
            } else { true }
        }
        _ => false
    })]
    #[ensures((^self).elems() == set!{&v} + self.elems())]
    fn insert(&mut self, v: i32) {
      match self {
        Tree::Leaf => {
          let right = Box::new(Tree::Leaf);
          let left = Box::new(Tree::Leaf);
          let new = Tree::Node { f: v as i32, left, right };
          *self = new
        }
        Tree::Node { f, left, right } => {
          let de = *f;
          if v < de { left.insert(v) } else if de < v { right.insert(v) }
        }
      }
    }
}

enum OptionCustom { Left { left: i16 }, Right { right: i16 } }
#[requires(match *i {
    OptionCustom::Left { left } => left < i16::MAX,
    OptionCustom::Right { right } => right < i16::MAX
})]
#[ensures(match (i, result) {
    (OptionCustom::Left { left }, OptionCustom::Right { right }) => *left+1 == right,
    (OptionCustom::Right { right }, OptionCustom::Left { left }) => left == *right+1,
    _ => false,
})]
fn swap_enums(i: &OptionCustom) -> OptionCustom {
  match i {
    OptionCustom::Left { left } => {
      let de = *left;
      OptionCustom::Right { right: (de + 1) as i16 }
    }
    OptionCustom::Right { right } => {
      let de = *right;
      OptionCustom::Left { left: (de + 1) as i16 }
    }
  }
}

#[requires(^x == 1)]
#[ensures(*x == 999)] // as hard as `false`
fn find_false(x: &mut i32) {
  *x = 0 as i32;
  unreachable!()
}

#[ensures(if cond { left === result && *right == ^right }
    else { right === result && *left == ^left }
)]
fn rbrrw_choice<'a>(left: &'a mut i32, right: &'a mut i32, cond: bool) -> &'a mut i32 {
  if cond { left } else { right }
}

struct End { x: i32, y: Common, }
struct Common { f3: u8, f4: i32 }
struct Start { z: Enum }
enum Enum { V1 { f5: Common }, V2 { f6: i32, f7: Common } }

#[requires(matches!(x.z, Enum::V2 { f6, .. } if f6 >= i.f4 && i.f4 >= 0))]
#[ensures(match (&x).z {
    Enum::V1 { f5: ref f5@Common { f3, .. } } => f5.f3 == f3,
    Enum::V2 { f6, .. } => result.x == f6 - i.f4,
})]
fn rearrange(i: &mut Common, x: Start) -> End {
  let de = i.f4;
  match x.z {
    Enum::V1 { .. } => unreachable!(),
    Enum::V2 { f6, f7 } => End { x: (f6 - de) as i32, y: f7 },
  }
}

/* ########## */
/* ##### Linked list tutorial benchmarks ##### */
/* ########## */

type Link<T> = Option<Box<NodeSLL<T>>>;
pub struct ListSLL<T> { head: Link<T> }
struct NodeSLL<T> { elem: T, next: Link<T> }

impl<T> NodeSLL<T> {
    #[pure]
    #[trusted_ensures(result >= 1)]
    pub fn len(&self) -> u16 {
        1 + (match &self.next {
            None => 0,
            Some(n) => n.len(),
        })
    }
}

impl<T> ListSLL<T> {
    #[ensures(result.len() == 0)]
    pub fn new() -> Self {
      ListSLL { head: None }
    }

    #[pure]
    #[trusted_ensures(result >= 0)]
    pub fn len(&self) -> u16 {
        match &self.head {
            None => 0,
            Some(n) => n.len(),
        }
    }

    // Alternative annotation commented out:
    // #[ensures((^self).len() == self.len() + 1)]
    #[ensures(match &(^self).head {
        // Some(node) => node.elem === elem,
        Some(node) => node.next === self.head,
        None => false,
    })]
    pub fn push(&mut self, elem: T) {
      let result = take(&mut self.head);
      let bx = NodeSLL { elem, next: result };
      let _0 = Box::new(bx);
      let new = Some(_0);
      self.head = new
    }

    // Alternative annotation commented out:
    // #[ensures(match (&result, &self.head) {
    //     (Some(v), Some(node)) => (^self).head === node.next && v === node.elem,
    //     (None, None) => true,
    //     _ => false,
    // })]
    #[ensures(match &self.head {
      Some(node) => (^self).head === node.next,
      None => true,
    })]
    pub fn pop(&mut self) -> Option<T> {
      let result = take(&mut self.head);
      match result {
        None => None,
        Some(_0) => {
          let result = Some(_0.elem);
          self.head = _0.next;
          result
        }
      }
    }

    // Alternative annotation commented out:
    // #[ensures(match (result, &self.head) {
    //     (Some(v), Some(node)) => v === node.elem,
    //     (None, None) => true,
    //     _ => false,
    // })]
    #[ensures(is_some(&self.head) == is_some(&result))]
    pub fn peek(&self) -> Option<&T> {
      match &self.head {
        None => None,
        Some(_0) => Some(&_0.elem),
      }
    }


    #[ensures(match (&(*self).head, result, &(^self).head) {
        (Some(node), Some(v), Some(fut)) =>
            node.elem === *v && ^v === fut.elem,
        (None, None, None) => true,
        _ => false,
    })]
    pub fn peek_mut(&mut self) -> Option<&mut T> {
      match &mut self.head {
        None => None,
        Some(_0) => Some(&mut _0.elem),
      }
    }
}

pub struct Iter<'a, T> { next: Option<&'a NodeSLL<T>> }
impl<'a, T> Iter<'a, T> {
    #[pure]
    #[trusted_ensures(result >= 0)]
    pub fn len(&self) -> u16 {
        match &self.next {
            None => 0,
            Some(n) => n.len(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    #[ensures(match self.len() {
        0 => (^self).len() == 0 && matches!(result, None),
        v => (^self).len() == v-1 && matches!(result, Some(_)),
    })]
    fn next(&mut self) -> Option<&'a T> {
      let (new, result) = match &mut self.next {
        None => (None, None),
        Some(_0) => match &_0.next {
            None => {
              let result = Some(&_0.elem);
              (None, result)
            }
            Some(_0_next_de) => {
              let result = Some(&_0.elem);
              let new = Some(&**_0_next_de);
              (new, result)
            }
          },
      };
      self.next = new;
      result
    }
}

pub struct IterMut<'a, T> { next: Option<&'a mut NodeSLL<T>> }
impl<'a, T> IterMut<'a, T> {
    #[pure]
    #[trusted_ensures(result >= 0)]
    pub fn len(&self) -> u16 {
        match &self.next {
            None => 0,
            Some(n) => n.len(),
        }
    }
}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    // The annotation here is incomplete:
    #[ensures(match (&self.next, result) {
        (None, None) => true,
        (Some(node), Some(result)) => (^*node).elem === ^result,
        _ => false,
    })]
    fn next(&mut self) -> Option<&'a mut T> {
      let result = take(&mut self.next);
      match result {
        None => None,
        Some(_0) => Some(&mut _0.elem),
      }
    }
}

// A custom benchmark that fits better here
impl NodeSLL<i32> {
    #[pure]
    fn sum(&self) -> i32 {
        self.elem +
        match &self.next {
            None => 0,
            Some(next) => next.sum(),
        }
    }

    #[ensures(match (&list.head, &(^list).head)  {
        (None, None) => true,
        (Some(list), Some(fut)) => list.len() == fut.len(),
        _ => false,
    })]
    #[ensures(match &(^list).head {
        None => true,
        Some(list) => list.sum() == (list.len() as i32) * val
    })]
    pub fn listset(list: &mut ListSLL<i32>, val: i32) {
      Self::listset_6(val, &mut list.head)
    }
    #[helper] fn listset_6(val: i32, head: &mut Option<Box<NodeSLL<i32>>>) {
      match head {
        None => (),
        Some(_0) => {
            Self::listset_6(val, &mut _0.next);
          _0.elem = val as i32
        }
      }
    }
}

/* ########## */
/* ##### StackOverflow benchmarks ##### */
/* ########## */

// https://stackoverflow.com/q/32165917
fn use_ref_ref<'a, 'b>(reference: &'a &'b mut ()) -> private_2::Token {
    private_2::use_same_ref_ref(reference)
}
mod private_2 {
    use russol_contracts::*;
    pub struct Token(());
    #[extern_spec]
    pub fn use_same_ref_ref<'c>(reference: &'c &'c mut ()) -> Token {
        Token(())
    }
}

// https://stackoverflow.com/q/22282117
struct Bar<T> { data: Option<Box<T>> }
impl<T> Bar<T> {
    #[ensures(is_some(&self.data) == is_ok(&result))]
    fn borrow(&mut self) -> Result<&Box<T>, ()> {
      match &mut self.data {
        None => Err(()),
        Some(_0) => Ok(_0),
      }
    }
}

// https://stackoverflow.com/q/52031002
struct SomeStruct<T> { attrib: T, next_attrib: Option<T> }
impl<T> SomeStruct<T> {
    #[ensures(if let Some(attr) = &self.next_attrib { (^self).attrib === attr } else { true })]
    pub fn apply_changes(&mut self) {
      let result = replace(&mut self.next_attrib, None);
      let new = match result {
        None => None,
        Some(_0) => {
          let result = replace(&mut self.attrib, _0);
          Some(result)
        }
      };
      self.next_attrib = new
    }
}

// https://stackoverflow.com/q/28258548
pub struct LinkedList { head: Option<Box<LinkedListNode>> }
pub struct LinkedListNode { next: Option<Box<LinkedListNode>> }
impl LinkedList {
    #[ensures(match &(^self).head {
        None => false,
        Some(lln) => lln.next === self.head
    })]
    pub fn prepend_value(&mut self) {
      let result = replace(&mut self.head, None);
      let bx = LinkedListNode { next: result };
      let _0 = Box::new(bx);
      let new = Some(_0);
      self.head = new
    }
}

// https://stackoverflow.com/q/29570781
enum Foo3<T> { Bar(T), Baz(T) }
impl<T> Foo3<T> {
    // [FAILURE] (requires unsafe)
    // fn switch(&mut self) {}
}

/* ########## */
/* ##### SuSLik benchmarks ##### */
/* ########## */

// Integers
#[ensures(^x == *y)]
#[ensures(^y == *x)]
fn int_swap(x: &mut i32, y: &mut i32) {
    let de_y = *y;
    let de_x = *x;
    *x = de_y as i32;
    *y = de_x as i32
}

// Singly linked list
impl<T> Node<T> {
    #[ensures(result.len() == self.len() + x2.len())]
    fn sll_append_copy(&self, x2: &Self) -> Self where T: Copy {
      match x2 {
        Node::Nil => self.sll_append_copy_7(),
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = self.sll_append_copy(&**next);
          let next = Box::new(result);
          Node::Cons { elem: de, next }
        }
      }
    }
    #[helper] fn sll_append_copy_7(&self) -> Node<T> {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = next.sll_append_copy_7();
          let next = Box::new(result);
          Node::Cons { elem: de, next }
        }
      }
    }

    #[ensures((^self).len() == (*self).len() + tl.len())]
    fn sll_append(&mut self, tl: Self) {
      match self {
        Node::Nil => *self = tl,
        Node::Cons { next, .. } => next.sll_append(tl),
      }
    }

    #[ensures(*self === result)]
    fn sll_copy(&self) -> Self where T: Copy {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = next.sll_copy();
          let next = Box::new(result);
          Node::Cons { elem: de, next }
        }
      }
    }

    // [REQUIRES] branch abduction
    // fn sll_delete_all(self, v: T) -> Self where T: Eq { }

    // [REQUIRES] branch abduction
    // fn sll_diff(x: &Node<i32>, y: &Node<i32>) -> Node<i32> { }

    // [UNINTERESTING]
    // sll_free{,2}

    #[ensures((^self).len() == self.len())]
    #[ensures((^self).elems() <= set!{ &v })]
    fn sll_init(&mut self, v: T) where T: Copy {
      let new = self.sll_init_3(v);
      *self = new
    }
    #[helper] fn sll_init_3(&mut self, v: T) -> Node<T> {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { next, .. } => {
          let new = next.sll_init_3(v);
          let next = Box::new(new);
          Node::Cons { elem: v, next }
        }
      }
    }

    // [REQUIRES] branch abduction
    // fn sll_intersect(x: &Node<i32>, y: &Node<i32>) -> Node<i32> { }

    #[requires(self.len() <= u16::MAX)]
    #[ensures(result == self.len())]
    fn sll_len(&self) -> u16 {
      match self {
        Node::Nil => 0 as u16,
        Node::Cons { next, .. } => {
          let result = next.sll_len();
          (result + 1) as u16
        }
      }
    }

    // [REQUIRES] branch abduction
    // fn sll_{max,min}(&self) -> &T where T: Ord { }

    #[ensures((^self).len() == (*self).len() + y.len() + z.len())]
    fn sll_append3(&mut self, y: Self, z: Self) {
      match self {
        Node::Nil => {
          let new = Self::sll_append3_7(y, z);
          *self = new
        }
        Node::Cons { next, .. } => next.sll_append3(y, z),
      }
    }
    #[helper] fn sll_append3_7(y: Node<T>, z: Node<T>) -> Node<T> {
      match y {
        Node::Nil => z,
        Node::Cons { elem, next } => {
          let new = Self::sll_append3_7(z, *next);
          let next = Box::new(new);
          Node::Cons { elem, next }
        }
      }
    }
    
    fn sll_singleton(elem: T) -> Self {
      let next = Box::new(Node::Nil);
      Node::Cons { elem, next }
    }

    // [REQUIRES] branch abduction
    // fn sll_union(...) -> Self { }

    // [REQUIRES] branch abduction
    // fn sll_unique(x: Node<i32>, ghost: u16) -> Node<i32> { }
}

// Sorted list
impl Node<u16> {
    #[pure]
    fn is_sorted(&self) -> bool {
        match self {
            Node::Nil | Node::Cons { next: box Node::Nil, .. } => true,
            Node::Cons { elem, next: box next@Node::Cons { elem: e, .. } } =>
                *elem <= *e && next.is_sorted(),
        }
    }

    #[extern_spec]
    #[requires(self.is_sorted())]
    #[ensures(result.is_sorted())]
    #[ensures(result.elems() == self.elems() + set!{ &v })]
    fn srtl_insert(self, v: u16) -> Self { todo!() }

    #[ensures(result.elems() == self.elems())]
    #[ensures(result.is_sorted())]
    fn insertion_sort(&self) -> Self {
      match self {
        Node::Nil => Node::Nil,
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = next.insertion_sort();
          result.srtl_insert(de)
        }
      }
    }

    // [REQUIRES] branch abduction
    // #[requires(self.is_sorted() && other.is_sorted())]
    // #[ensures(result.elems() == self.elems() + other.elems())]
    // #[ensures(result.is_sorted())]
    // fn srtl_merge(self, other: Self) -> Self { }

    // Not a full spec (due to lack of intervals support):
    #[requires(if let Node::Cons { elem, .. } = self { v <= elem } else { true })]
    #[ensures(result.len() == self.len() + 1)]
    fn srtl_prepend(self, v: u16) -> Self {
      let next = Box::new(self);
      Node::Cons { elem: v as u16, next }
    }

    // [REQUIRES] intervals (for proper sortedness property)
    // fn srtl_rev(self) -> Self { }
}

#[derive(Copy, Clone)]
struct Token<'a>(&'a Token<'a>);
impl<T> Node<T> {
    #[extern_spec]
    #[ensures(result.elems() == self.elems() + other.elems())]
    fn append(self, other: Self, token: Token) -> Self {
        todo!()
    }
}

// Binary tree
impl<T> Tree<T> {
    #[ensures(result.elems() == self.elems())]
    fn tree_copy(&self) -> Self where T: Copy {
      match self {
        Tree::Leaf => Tree::Leaf,
        Tree::Node { f, left, right } => {
          let de = *f;
          let result_1 = left.tree_copy();
          let result_2 = right.tree_copy();
          let right = Box::new(result_2);
          let left = Box::new(result_1);
          Tree::Node { f: de, left, right }
        }
      }
    }

    #[ensures((^l).elems() == self.elems() + l.elems())]
    #[params("--closeWhileAbduce=false")]
    fn tree_flatten_acc(self, l: &mut Node<T>) {
      match l {
        Node::Nil => {
          let new = self.tree_flatten_acc_7();
          *l = new
        }
        Node::Cons { next, .. } => self.tree_flatten_acc(&mut **next), // <- TODO: investigate why the args here get swapped
      }
    }
    #[helper] fn tree_flatten_acc_7(self) -> Node<T> {
      match self {
        Tree::Leaf => Node::Nil,
        Tree::Node { f, left, right } => {
          let new = left.tree_flatten_acc_7();
          Self::tree_flatten_acc_18(f, *right, new)
        }
      }
    }
    #[helper] fn tree_flatten_acc_18(f: T, bx: Tree<T>, new: Node<T>) -> Node<T> {
      match new {
        Node::Nil => {
          let new = bx.tree_flatten_acc_7();
          let next = Box::new(new);
          Node::Cons { elem: f, next }
        }
        Node::Cons { elem, next } => {
          let new = Self::tree_flatten_acc_18(elem, bx, *next);
          let next = Box::new(new);
          Node::Cons { elem: f, next }
        }
      }
    }

    // [UNSUPPORTED] owned aliasing
    // fn tree_dll{,_linear}

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    fn tree_flatten_helper(self, token: Token) -> Node<T> {
      match self {
        Tree::Leaf => Node::Nil,
        Tree::Node { f, left, right } => {
          let result_1 = left.tree_flatten_helper(token);
          let result_2 = right.tree_flatten_helper(token);
          let result = result_1.append(result_2, token);
          let next = Box::new(result);
          Node::Cons { elem: f, next }
        }
      }
    }

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    fn tree_flatten(self) -> Node<T> {
      match self {
        Tree::Leaf => Node::Nil,
        Tree::Node { f, left, right } => {
          let result = left.tree_flatten();
          Self::tree_flatten_12(f, *right, result)
        }
      }
    }
    #[helper] fn tree_flatten_12(f: T, bx: Tree<T>, result: Node<T>) -> Node<T> {
      match result {
        Node::Nil => {
          let result = bx.tree_flatten();
          let next = Box::new(result);
          Node::Cons { elem: f, next }
        }
        Node::Cons { elem, next } => {
          let result = Self::tree_flatten_12(elem, bx, *next);
          let next = Box::new(result);
          Node::Cons { elem: f, next }
        }
      }
    }

    // [UNINTERESTING]
    // fn free{,2}

    #[requires(self.size() <= u16::MAX)]
    #[ensures(result == self.size())]
    fn tree_size(&self) -> u16 {
      match self {
        Tree::Leaf => 0 as u16,
        Tree::Node { left, right, .. } => {
          let result_1 = left.tree_size();
          let result_2 = right.tree_size();
          (result_1 + result_2 + 1) as u16
        }
      }
    }
}

enum RoseTree<T> { Nil, Cons { elem: T, next: ListRT<RoseTree<T>> } }
enum ListRT<T> {
    Nil,
    Cons(Box<(T, ListRT<T>)>),
}
impl<T> ListRT<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            ListRT::Nil => set!{},
            ListRT::Cons(box (hd, tl)) => tl.elems() + set!{ hd },
        }
    }
    #[pure]
    #[trusted_ensures(result >= 0 && result <= u16::MAX)]
    fn len(&self) -> u16 {
        match self {
            ListRT::Nil => 0,
            ListRT::Cons(box (_, tl)) => 1 + tl.len(),
        }
    }
}
impl<T> ListRT<RoseTree<T>> {
    #[pure]
    fn elems_tree(&self) -> Set<T> {
        match self {
            ListRT::Nil => set!{},
            ListRT::Cons(box (hd, tl)) => hd.elems() + tl.elems_tree(),
        }
    }
}

// Rose tree
impl<T> RoseTree<T> {
    #[pure]
    fn elems(&self) -> Set<T> {
        match self {
            RoseTree::Nil => set!{},
            RoseTree::Cons { elem, next } => next.elems_tree() + set!{ elem },
        }
    }

    #[ensures(result.elems() == self.elems())]
    fn copy(&self) -> Self where T: Copy {
        match self {
          RoseTree::Nil => RoseTree::Nil,
          RoseTree::Cons { elem, next } => {
            let de = *elem;
            Self::copy_8(de, elem, next)
          }
        }
    }
    #[helper] fn copy_8(de: T, elem_self: &T, next: &ListRT<RoseTree<T>>) -> RoseTree<T> where T: Copy {
        match next {
          ListRT::Nil => RoseTree::Cons { elem: de, next: ListRT::Nil },
          ListRT::Cons(_0) => {
            let result = _0.0.copy();
            match result {
              RoseTree::Nil => Self::copy_8(de, elem_self, &_0.1),
              RoseTree::Cons { elem: elem_result, next } => {
                let result = Self::copy_8(de, elem_self, &_0.1);
                let bx = (result, next);
                let _0 = Box::new(bx);
                let next = ListRT::Cons(_0);
                RoseTree::Cons { elem: elem_result, next }
              }
            }
          }
        }
    }

    #[ensures(result.elems() == self.elems())]
    #[params("--closeWhileAbduce=false")]
    fn flatten(self) -> ListRT<T> {
        match self {
            RoseTree::Nil => ListRT::Nil,
            RoseTree::Cons { elem, next } => Self::flatten_3(elem, next)
        }
    }
    #[helper] fn flatten_3(elem: T, next: ListRT<RoseTree<T>>) -> ListRT<T> {
        match next {
            ListRT::Nil => {
              let bx = (elem, ListRT::Nil);
              let _0 = Box::new(bx);
              ListRT::Cons(_0)
            }
            ListRT::Cons(_0) => {
              let result = _0.0.flatten();
              Self::flatten_14(elem, _0.1, result)
            }
        }
    }
    #[helper] fn flatten_14(elem: T, _1: ListRT<RoseTree<T>>, result: ListRT<T>) -> ListRT<T> {
        match result {
          ListRT::Nil => Self::flatten_3(elem, _1),
          ListRT::Cons(_0) => {
            let result = Self::flatten_14(_0.0, _1, _0.1);
            let bx = (elem, result);
            let _0 = Box::new(bx);
            ListRT::Cons(_0)
          }
        }
    }
}

// List of lists
impl<T> ListRT<ListRT<T>> {
    #[pure]
    fn elems_list(&self) -> Set<T> {
        match self {
            ListRT::Nil => set!{},
            ListRT::Cons(box (hd, tl)) => hd.elems() + tl.elems_list(),
        }
    }
    #[pure]
    fn mlen(&self) -> u16 {
        match self {
            ListRT::Nil => 0,
            ListRT::Cons(box (hd, tl)) => hd.len() + tl.mlen(),
        }
    }

    #[ensures(result.elems() == self.elems_list())]
    #[params("--closeWhileAbduce=false")]
    fn flatten(self) -> ListRT<T> {
      match self {
        ListRT::Nil => ListRT::Nil,
        ListRT::Cons(_0) => Self::flatten_7(_0.0, _0.1),
      }
    }
    #[helper] fn flatten_7(_0: ListRT<T>, _1: ListRT<ListRT<T>>) -> ListRT<T> {
      match _0 {
        ListRT::Nil => _1.flatten(),
        ListRT::Cons(_0) => {
          let result = Self::flatten_7(_0.1, _1);
          let bx = (_0.0, result);
          let _0 = Box::new(bx);
          ListRT::Cons(_0)
        }
      }
    }

    // [FAILURE] pure synthesis
    // #[requires(self.mlen() <= u16::MAX)]
    // #[ensures(result == self.mlen())]
    // fn multilist_length(&self) -> u16 { }
}


/* ########## */
/* ##### Creusot benchmarks ##### */
/* ########## */

impl Node<u16> {
    #[pure]
    fn sum(&self) -> u16 {
        match self {
            Node::Cons { elem, next } => *elem + next.sum(),
            Node::Nil => 0,
        }
    }

    #[requires(self.sum() <= u16::MAX)]
    #[ensures(result == self.sum())]
    fn sum_x(&self) -> u16 {
      match self {
        Node::Nil => 0 as u16,
        Node::Cons { elem, next } => {
          let de = *elem;
          let result = next.sum_x();
          (de + result) as u16
        }
      }
    }

    #[requires(self.sum() > 0)]
    #[ensures((^self).sum() - self.sum() ==
        ^result.0 + (^result.1).sum() - *result.0 - (*result.1).sum())]
    fn take_some_rest(&mut self) -> (&mut u16, &mut Node<u16>) {
      match self {
        Node::Nil => unreachable!(),
        Node::Cons { elem, next } => (elem, &mut **next),
      }
    }
}

#[ensures(result.0 === x.1 && result.1 === x.0)]
fn swap<T>(x: (T, T)) -> (T, T) {
  (x.1, x.0)
}

#[ensures(if *ma >= *mb { *mb == ^mb && result === ma }
                    else { *ma == ^ma && result === mb })]
fn take_max<'a>(ma: &'a mut u16, mb: &'a mut u16) -> &'a mut u16 {
  let de_mb = *mb;
  let de_ma = *ma;
  if de_mb <= de_ma { ma } else { mb }
}

#[ensures(*result == **x)]
#[ensures(^result == *^x)]
#[ensures(^*x == ^^x)]
pub fn unnest<'a, 'b: 'a>(x: &'a mut &'b mut i32) -> &'a mut i32 {
  &mut **x
}

#[extern_spec]
#[ensures(result == if a + b >= 256 { a + b - 256 } else { a + b })]
pub fn wrapping_add(a: u8, b: u8) -> u8 {
    a.wrapping_add(b)
}
#[ensures(result == a + b || result == a + b - 256)]
pub fn test_u8_wrapping_add(a: u8, b: u8) -> u8 {
  let result = wrapping_add(b as u8, a);
  result
}

/* ########## */
/* ##### Prusti benchmarks ##### */
/* ########## */

struct Account { bal: u16 }
impl Account {
    #[pure]
    fn balance(&self) -> u16 {
        self.bal
    }

    #[requires(self.balance() + amount <= u16::MAX)]
    #[ensures((^self).balance() == self.balance() + amount)]
    fn deposit(&mut self, amount: u16) {
      let de = self.bal;
      let new = Account { bal: (de + amount) as u16 };
      *self = new
    }

    #[requires(amount <= self.balance())]
    #[ensures((^self).balance() == self.balance() - amount)]
    fn withdraw(&mut self, amount: u16) {
      let de = self.bal;
      let new = Account { bal: (de - amount) as u16 };
      *self = new
    }

    #[requires(other.balance() + amount <= u16::MAX)]
    #[requires(amount <= self.balance())]
    #[ensures((^self).balance() == self.balance() - amount)]
    #[ensures((^other).balance() == other.balance() + amount)]
    fn transfer(&mut self, other: &mut Account, amount: u16) {
      let de_bal_other = other.bal;
      let de_bal_self = self.bal;
      self.bal = (de_bal_self - amount) as u16;
      other.bal = (de_bal_other + amount) as u16
    }
}

#[ensures(result == *my_box)]
fn foo(my_box: Box<i32>) -> i32 {
  *my_box
}


pub enum TreePrusti { Node(i16, Box<TreePrusti>, Box<TreePrusti>), Empty }

impl TreePrusti {
    #[pure]
    pub fn rightmost(&self) -> i16 {
        match self {
            TreePrusti::Empty => i16::MAX,
            TreePrusti::Node(value, _, box TreePrusti::Empty) => *value,
            TreePrusti::Node(_, _, right) => right.rightmost(),
        }
    }
    #[pure]
    pub fn leftmost(&self) -> i16 {
        match self {
            TreePrusti::Empty => i16::MIN,
            TreePrusti::Node(value, box TreePrusti::Empty, _) => *value,
            TreePrusti::Node(_, left, _) => left.leftmost(),
        }
    }
    #[pure]
    pub fn bst_invariant(&self) -> bool {
        if let TreePrusti::Node(value, left, right) = self {
            (if let TreePrusti::Node(..) = &**left { *value >= left.rightmost() } else { true }) &&
            (if let TreePrusti::Node(..) = &**right { *value <= right.leftmost() } else { true }) &&
            left.bst_invariant() && right.bst_invariant()
        } else { true }
    }

    #[requires(self.bst_invariant())]
    #[ensures((^self).bst_invariant())]
    #[requires(
        if let TreePrusti::Node(_, left, right) = &*self {
            (if let TreePrusti::Node(..) = &**left { ^result >= left.rightmost() } else { true }) &&
            (if let TreePrusti::Node(..) = &**right { ^result <= right.leftmost() } else { true })
        } else { false }
    )]
    pub fn get_root_value(&mut self) -> &mut i16 {
      match self {
        TreePrusti::Node(_0, _, _) => _0,
        TreePrusti::Empty => unreachable!(),
      }
    }
}

struct S { a: i32, b: i32 }
#[requires(x.0 == 123 && x.1 == 42)]
#[ensures(result.0 == 42 && result.1 == 123)]
fn test_tuple_field(x: (i32, i32)) -> (i32, i32) {
  (42 as i32, 123 as i32)
}
#[requires(x.a == 123 && x.b == 42)]
#[ensures(result.a == 42 && result.b == 123)]
fn test_struct_field(x: S) -> S {
  S { a: 42 as i32, b: 123 as i32 }
}

struct Number<A, B, C> { a: A, b: B, c: C }
#[requires(-10_000 < arg.b && arg.b < 10_000)]
#[ensures((^arg).b == arg.b - 1000)]
fn test1<A, B>(arg: &mut Number<A, i32, B>) {
  let de = arg.b;
  arg.b = (de - 1000) as i32
}
#[requires(-10_000 < arg.b.b && arg.b.b < 10_000)]
#[ensures((^arg).b.b == arg.b.b - 1000)]
fn test2<A, B, C, D>(arg: &mut Number<A, Number<B, i32, C>, D>) {
  let de = arg.b.b;
  arg.b.b = (de - 1000) as i32
}

struct Foo4<A> { i: i32, x: BarBaz<A> }
struct BarBaz<B> { i: i32, x: ::std::marker::PhantomData<B> }
#[ensures(result.i == arg.x.i)]
#[ensures(result.x.i == arg.i)]
fn test1_alt<C, D>(arg: Foo4<C>) -> Foo4<D> {
  let x = BarBaz { i: arg.i as i32, x: ::std::marker::PhantomData };
  Foo4 { i: arg.x.i as i32, x }
}

#[requires(x == -42)]
#[ensures(match result { Some(..) => true, _ => false })]
fn test_match_expr(x: i32) -> Option<i32> {
  Some(0 as i32)
}

#[requires(x == 42)]
#[ensures(match result { 84 => true, 123 | 456 => false, _ => false })]
fn test_match_expr_2(x: u32) -> u32 {
  84 as u32
}
#[requires(x == -42)]
#[ensures(match result { Some(k) => k == -42, _ => false })]
fn test_match_option_expr(x: i32) -> Option<i32> {
  Some((-42) as i32)
}

struct Ts { f: u32 }
#[ensures(^x == 4)]
fn test1_alt1(x: &mut u32) {
  *x = 4 as u32
}
#[ensures((^x).f == 4)]
fn test2_alt1(x: &mut Ts) {
  let new = Ts { f: 4 as u32 };
  *x = new
}

struct Point { x: Box<u16>, y: Box<u16> }
#[requires(u16::MAX - *a >= b)]
#[ensures(*result == *a + b)]
fn add(a: Box<u16>, b: u16) -> Box<u16> {
  Box::new((*a + b) as u16)
}
#[requires(u16::MAX - *p.x >= s)]
#[ensures(*result.x == *p.x + s)]
#[ensures(*result.y == *p.y)]
fn shift_x(p: Point, s: u16) -> Point {
  let x = Box::new((*p.x + s) as u16);
  Point { x, y: p.y }
}

#[requires(*x == 5)]
#[ensures(*x == 5)]
pub fn test6(x: &u32) {
  ()
}
pub fn test_alt2<T>(x: &'static T) -> &'_ T {
  x
}

pub fn u32_i64(x: u32) -> i64 {
  x as i64
}
pub fn u32_isize(x: u32) -> isize {
  x as isize
}
#[requires(x < std::i32::MAX as u64)]
pub fn u64_u32(x: u64) -> i32 {
  x as i32
}
#[requires(x < std::i16::MAX as u64)]
pub fn u64_i16(x: u64) -> i16 {
  x as i16
}
#[requires(x < std::i8::MAX as u64)]
pub fn u64_i8(x: u64) -> i8 {
  x as i8
}
#[requires(x < std::i8::MAX as u16)]
pub fn u16_i8(x: u16) -> i8 {
  x as i8
}
#[requires(x < std::u16::MAX as i64)]
#[requires(0 <= x)]
pub fn i64_u16(x: i64) -> u16 {
  x as u16
}
#[requires(x < std::u8::MAX as i64)]
#[requires(0 <= x)]
pub fn i64_u8(x: i64) -> u8 {
  x as u8
}
#[requires(x < std::u8::MAX as i16)]
#[requires(0 <= x)]
pub fn i16_u8(x: i16) -> u8 {
  x as u8
}

pub fn i32_i64(x: i32) -> i64 {
  x as i64
}
pub fn i32_isize(x: i32) -> isize {
  x as isize
}
#[requires(x < std::i16::MAX as i64)]
#[requires(std::i16::MIN as i64 <= x)]
pub fn i64_i16(x: i64) -> i16 {
  0 as i16
}
#[requires(x < std::i8::MAX as i64)]
#[requires(std::i8::MIN as i64 <= x)]
pub fn i64_i8(x: i64) -> i8 {
  0 as i8
}
#[requires(x < std::i8::MAX as i16)]
#[requires(std::i8::MIN as i16 <= x)]
pub fn u16_i8_2(x: i16) -> i8 {
  0 as i8
}

pub fn u32_u64(x: u32) -> u64 {
  x as u64
}
pub fn u32_usize(x: u32) -> usize {
  x as usize
}
#[requires(x < std::u16::MAX as u64)]
pub fn u64_u16(x: u64) -> u16 {
  x as u16
}
#[requires(x < std::u8::MAX as u64)]
pub fn u64_u8(x: u64) -> u8 {
  x as u8
}
#[requires(x < std::u8::MAX as u16)]
pub fn u16_u8(x: u16) -> u8 {
  x as u8
}

struct Ta { val: i32 }

#[ensures((^x).val == (^result).val)]
fn identity(x: &mut Ta) -> &mut Ta {
  x
}
#[ensures(result.val == v)]
#[ensures((^x).val == (^result).val)]
fn identity2(x: &mut Ta, v: i32) -> &mut Ta {
  let new = Ta { val: v };
  *x = new;
  x
}
#[ensures(*result == v)]
#[ensures((^x).val == (^result))]
fn identity3(x: &mut Ta, v: i32) -> &mut i32 {
  x.val = v as i32;
  &mut x.val
}

/* ########## */
/* ##### Standard library function
    annotations provided by the tool ##### */
/* ########## */

#[extern_spec]
#[ensures(matches!(^x, None))]
#[ensures(*x === result)]
fn take<T>(x: &mut Option<T>) -> Option<T> { x.take() }

#[extern_spec]
#[ensures(*dest === result)]
#[ensures(^dest === src)]
fn replace<T>(dest: &mut T, src: T) -> T { std::mem::replace(dest, src) }

#[pure]
fn is_some<T>(o: &Option<T>) -> bool { matches!(o, Some(_)) }
#[pure]
fn is_none<T>(opt: &Option<T>) -> bool { matches!(opt, None) }

#[pure]
fn is_ok<T, E>(x: &Result<T, E>) -> bool {
    matches!(x, Ok(_))
}
