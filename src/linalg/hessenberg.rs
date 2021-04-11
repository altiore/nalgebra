#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

use crate::allocator::Allocator;
use crate::base::{DefaultAllocator, OMatrix, OVector};
use crate::dimension::{Const, DimDiff, DimSub, U1};
use crate::storage::Storage;
use simba::scalar::ComplexField;

use crate::linalg::householder;

/// Hessenberg decomposition of a general matrix.
#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde-serialize",
    serde(bound(serialize = "DefaultAllocator: Allocator<T, D, D> +
                           Allocator<T, DimDiff<D, U1>>,
         OMatrix<T, D, D>: Serialize,
         OVector<T, DimDiff<D, U1>>: Serialize"))
)]
#[cfg_attr(
    feature = "serde-serialize",
    serde(bound(deserialize = "DefaultAllocator: Allocator<T, D, D> +
                           Allocator<T, DimDiff<D, U1>>,
         OMatrix<T, D, D>: Deserialize<'de>,
         OVector<T, DimDiff<D, U1>>: Deserialize<'de>"))
)]
#[derive(Clone, Debug)]
pub struct Hessenberg<T: ComplexField, D: DimSub<U1>>
where
    DefaultAllocator: Allocator<T, D, D> + Allocator<T, DimDiff<D, U1>>,
{
    hess: OMatrix<T, D, D>,
    subdiag: OVector<T, DimDiff<D, U1>>,
}

impl<T: ComplexField, D: DimSub<U1>> Copy for Hessenberg<T, D>
where
    DefaultAllocator: Allocator<T, D, D> + Allocator<T, DimDiff<D, U1>>,
    OMatrix<T, D, D>: Copy,
    OVector<T, DimDiff<D, U1>>: Copy,
{
}

impl<T: ComplexField, D: DimSub<U1>> Hessenberg<T, D>
where
    DefaultAllocator: Allocator<T, D, D> + Allocator<T, D> + Allocator<T, DimDiff<D, U1>>,
{
    /// Computes the Hessenberg decomposition using householder reflections.
    pub fn new(hess: OMatrix<T, D, D>) -> Self {
        let mut work = unsafe {
            crate::unimplemented_or_uninitialized_generic!(hess.data.shape().0, Const::<1>)
        };
        Self::new_with_workspace(hess, &mut work)
    }

    /// Computes the Hessenberg decomposition using householder reflections.
    ///
    /// The workspace containing `D` elements must be provided but its content does not have to be
    /// initialized.
    pub fn new_with_workspace(mut hess: OMatrix<T, D, D>, work: &mut OVector<T, D>) -> Self {
        assert!(
            hess.is_square(),
            "Cannot compute the hessenberg decomposition of a non-square matrix."
        );

        let dim = hess.data.shape().0;

        assert!(
            dim.value() != 0,
            "Cannot compute the hessenberg decomposition of an empty matrix."
        );
        assert_eq!(
            dim.value(),
            work.len(),
            "Hessenberg: invalid workspace size."
        );

        let mut subdiag = unsafe {
            crate::unimplemented_or_uninitialized_generic!(dim.sub(Const::<1>), Const::<1>)
        };

        if dim.value() == 0 {
            return Hessenberg { hess, subdiag };
        }

        for ite in 0..dim.value() - 1 {
            householder::clear_column_unchecked(&mut hess, &mut subdiag[ite], ite, 1, Some(work));
        }

        Hessenberg { hess, subdiag }
    }

    /// Retrieves `(q, h)` with `q` the orthogonal matrix of this decomposition and `h` the
    /// hessenberg matrix.
    #[inline]
    pub fn unpack(self) -> (OMatrix<T, D, D>, OMatrix<T, D, D>) {
        let q = self.q();

        (q, self.unpack_h())
    }

    /// Retrieves the upper trapezoidal submatrix `H` of this decomposition.
    #[inline]
    pub fn unpack_h(mut self) -> OMatrix<T, D, D> {
        let dim = self.hess.nrows();
        self.hess.fill_lower_triangle(T::zero(), 2);
        self.hess
            .slice_mut((1, 0), (dim - 1, dim - 1))
            .set_partial_diagonal(self.subdiag.iter().map(|e| T::from_real(e.modulus())));
        self.hess
    }

    // TODO: add a h that moves out of self.
    /// Retrieves the upper trapezoidal submatrix `H` of this decomposition.
    ///
    /// This is less efficient than `.unpack_h()` as it allocates a new matrix.
    #[inline]
    pub fn h(&self) -> OMatrix<T, D, D> {
        let dim = self.hess.nrows();
        let mut res = self.hess.clone();
        res.fill_lower_triangle(T::zero(), 2);
        res.slice_mut((1, 0), (dim - 1, dim - 1))
            .set_partial_diagonal(self.subdiag.iter().map(|e| T::from_real(e.modulus())));
        res
    }

    /// Computes the orthogonal matrix `Q` of this decomposition.
    pub fn q(&self) -> OMatrix<T, D, D> {
        householder::assemble_q(&self.hess, self.subdiag.as_slice())
    }

    #[doc(hidden)]
    pub fn hess_internal(&self) -> &OMatrix<T, D, D> {
        &self.hess
    }
}
