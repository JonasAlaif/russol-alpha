use russol_contracts::*;

enum Avl<T> {
    Leaf,
    Node { f: T, left: Box<Avl<T>>, right: Box<Avl<T>> },
}

impl Avl<i32> {
    #[pure]
    fn size(&self) -> u16 {
        match self {
            Avl::Leaf => 0,
            Avl::Node { left, right, .. } => 1 + left.size() + right.size(),
        }
    }
    #[pure]
    fn height(&self) -> u16 {
        match self {
            Avl::Leaf => 0,
            Avl::Node { left, right, .. } => {
                let (lh, rh) = (left.height(), right.height());
                1 + if lh >= rh { lh } else { rh }
            }
        }
    }
    #[pure]
    fn is_avl(&self) -> bool {
        match self {
            Avl::Leaf => true,
            Avl::Node { left, right, .. } => {
                let (lh, rh) = (left.height(), right.height());
                lh <= rh+1 && rh <= lh+1 && left.is_avl() && right.is_avl()
            }
        }
    }
}

#[requires(x.is_avl())]
#[ensures(result.size() == x.size())]
#[ensures(result.height() == x.height())]
#[ensures(result.is_avl())]
fn avl_copy(x: &Avl<i32>) -> Avl<i32> {
  match x {
    Avl::Leaf => Avl::Leaf,
    Avl::Node { f, left, right } => {
      let de = *f;
      let result_1 = avl_copy(&**left);
      let result_2 = avl_copy(&**right);
      let right = Box::new(result_2);
      let left = Box::new(result_1);
      Avl::Node { f: de as i32, left, right }
    }
  }
}
