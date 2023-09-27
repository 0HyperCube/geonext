use std::ops::{Deref, DerefMut};

use glam::DVec2;

fn default<T: Default>() -> T {
	T::default()
}

struct LayoutSizes {
	main_size: f64,
	cross_size: f64,
	allocated_size: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct BoxConstraint {
	min: DVec2,
	max: DVec2,
}
impl BoxConstraint {
	pub fn new(min: impl Into<DVec2>, max: impl Into<DVec2>) -> Self {
		Self { min: min.into(), max: max.into() }
	}
	/// Maximum size and minimum size are equal (forcing the size)
	/// https://docs.flutter.dev/ui/layout/constraints#tight-constraints
	pub const fn tight(size: DVec2) -> Self {
		Self { min: size, max: size }
	}
	pub const fn tight_width(width: f64) -> Self {
		Self {
			min: DVec2::new(width, 0.),
			max: DVec2::new(width, f64::MAX),
		}
	}
	pub const fn tight_height(height: f64) -> Self {
		Self {
			min: DVec2::new(0., height),
			max: DVec2::new(f64::MAX, height),
		}
	}
	pub const fn max_width(mut self, width: f64) -> Self {
		self.max.x = width;
		self
	}
	pub const fn max_height(mut self, height: f64) -> Self {
		self.max.y = height;
		self
	}
	/// No minimum size but a specified maximum
	/// https://docs.flutter.dev/ui/layout/constraints#loose-constraints
	pub const fn loose(size: DVec2) -> Self {
		Self { min: DVec2::ZERO, max: size }
	}
	/// No minimum or maximum size.
	/// https://docs.flutter.dev/ui/layout/constraints#unbounded-constraints
	pub const fn unbounded() -> Self {
		Self { min: DVec2::ZERO, max: DVec2::MAX }
	}
	pub fn constrain_width(self, width: f64) -> f64 {
		width.clamp(self.min.x, self.max.x)
	}
	pub fn constrain_height(self, hight: f64) -> f64 {
		hight.clamp(self.min.y, self.max.y)
	}
	pub fn constrain(self, size: Size) -> Size {
		Size(size.0.clamp(self.min, self.max))
	}
}

#[derive(Clone, Copy, Default)]
pub struct Size(pub DVec2);
impl Deref for Size {
	type Target = DVec2;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for Size {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
/// How the child is inscribed into the available space.
#[derive(Default)]
pub enum FlexFit {
	/// The child is forced to fill the available space.
	Tight,
	/// The child can be at most as large as the available space (but is allowed to be smaller).
	#[default]
	Loose,
}

#[derive(Default)]
pub struct RenderParams {
	/// Flex factor.
	/// If 0, no flex is applied. If not zero, the space will be computed by dividing the free space after placing non-flexible children.
	pub flex: u32,
	pub fit: FlexFit,
	pub offset: DVec2,
	pub size: Size,
}

pub trait UiElement {
	fn layout(&mut self, box_constraint: BoxConstraint) -> Size;
	fn render_params(&mut self) -> &mut RenderParams;
}

pub struct Expanded<E: UiElement> {
	child: E,
	render_params: RenderParams,
}

impl<E: UiElement> UiElement for Expanded<E> {
	fn layout(&mut self, box_constraint: BoxConstraint) -> Size {
		self.child.layout(BoxConstraint::tight(box_constraint.max));
		Size(box_constraint.max)
	}
	fn render_params(&mut self) -> &mut RenderParams {
		self.render_params.fit = FlexFit::Tight;
		&mut self.render_params
	}
}

#[derive(Default)]
pub struct Container<E: UiElement> {
	child: E,
	render_params: RenderParams,
}

impl<E: UiElement> UiElement for Container<E> {
	fn layout(&mut self, box_constraint: BoxConstraint) -> Size {
		self.child.layout(box_constraint)
	}

	fn render_params(&mut self) -> &mut RenderParams {
		&mut self.render_params
	}
}

pub trait UiElementList {
	fn len(&self) -> usize;
	fn render_params(&mut self, index: usize) -> &mut RenderParams;
	fn layout_nth(&mut self, index: usize, box_contraint: BoxConstraint) -> Size;
}

struct ComputedSize(DVec2);

pub struct Flex<E: UiElementList> {
	pub children: E,
	pub direction: Axis,
	pub main_axis_alignment: MainAxisAlignment,
	pub main_axis_size: MainAxisSize,
	pub cross_axis_alignment: CrossAxisAlignment,
	computed_size: ComputedSize,
	pub render_params: RenderParams,
}

impl Default for Flex<()> {
	fn default() -> Self {
		Self {
			children: (),
			direction: Axis::default(),
			main_axis_alignment: MainAxisAlignment::default(),
			main_axis_size: MainAxisSize::default(),
			cross_axis_alignment: CrossAxisAlignment::default(),
			computed_size: ComputedSize(DVec2::ZERO),
			render_params: Default::default(),
		}
	}
}

impl<E: UiElementList> Flex<E> {
	fn compute_sizes(&mut self, constraints: BoxConstraint) -> LayoutSizes {
		let mut total_flex = 0;
		let max_main_size = constraints.max[self.direction];
		let can_flex = max_main_size < f64::INFINITY;

		let mut cross_size: f64 = 0.;
		let mut allocated_size = 0.; // sum of non-flexible children
		let mut last_flex_child = None;

		for child_index in 0..self.children.len() {
			let flex_data = self.children.render_params(child_index);
			let flex = flex_data.flex;
			if flex > 0 {
				total_flex += flex;
				last_flex_child = Some(child_index);
			} else {
				let inner_constraints = match (self.cross_axis_alignment, self.direction) {
					(CrossAxisAlignment::Stretch, Axis::Horizontal) => BoxConstraint::tight_height(constraints.max.y),
					(CrossAxisAlignment::Stretch, Axis::Vertical) => BoxConstraint::tight_width(constraints.max.x),
					(_, Axis::Horizontal) => BoxConstraint::unbounded().max_height(constraints.max.y),
					(_, Axis::Vertical) => BoxConstraint::unbounded().max_width(constraints.max.x),
				};
				let child_size = self.children.layout_nth(child_index, inner_constraints);
				allocated_size += child_size.0[self.direction];
				cross_size = cross_size.max(child_size.0[self.direction.cross()]);
				self.children.render_params(child_index).size = child_size;
			}
		}

		let free_space = 0_f64.max(if can_flex { max_main_size } else { 0. } - allocated_size);
		let mut allocated_flex_space = 0.;
		if total_flex > 0 {
			let space_per_flex = if can_flex { free_space / total_flex as f64 } else { f64::NAN };
			for child_index in 0..self.children.len() {
				let render_params = self.children.render_params(child_index);
				let flex = render_params.flex;
				if flex <= 0 {
					continue;
				}
				let max_child_extent = if can_flex {
					if last_flex_child == Some(child_index) {
						free_space - allocated_flex_space
					} else {
						space_per_flex * flex as f64
					}
				} else {
					f64::INFINITY
				};
				let min_child_extent = match render_params.fit {
					FlexFit::Tight => {
						assert!(max_child_extent < f64::INFINITY);
						max_child_extent
					}
					FlexFit::Loose => 0.,
				};
				let inner_constraints = match (self.cross_axis_alignment, self.direction) {
					(CrossAxisAlignment::Stretch, Axis::Horizontal) => BoxConstraint::new((min_child_extent, constraints.max.y), (max_child_extent, constraints.max.y)),
					(CrossAxisAlignment::Stretch, Axis::Vertical) => BoxConstraint::new((constraints.max.x, min_child_extent), (constraints.max.x, max_child_extent)),
					(_, Axis::Horizontal) => BoxConstraint::new((min_child_extent, 0.), (max_child_extent, constraints.max.y)),
					(_, Axis::Vertical) => BoxConstraint::new((0., min_child_extent), (constraints.max.x, max_child_extent)),
				};
				let child_size = self.children.layout_nth(child_index, inner_constraints);
				let child_main_size = child_size.0[self.direction];
				assert!(child_main_size <= max_child_extent);
				allocated_size += child_main_size;
				allocated_flex_space += max_child_extent;
				cross_size = cross_size.max(child_size.0[self.direction.cross()]);
				self.children.render_params(child_index).size = child_size;
			}
		}

		let ideal_size = if can_flex && self.main_axis_size == MainAxisSize::Max { max_main_size } else { allocated_size };
		LayoutSizes {
			main_size: ideal_size,
			cross_size,
			allocated_size,
		}
	}
}

impl<E: UiElementList> UiElement for Flex<E> {
	fn layout(&mut self, constraints: BoxConstraint) -> Size {
		let LayoutSizes {
			mut main_size,
			mut cross_size,
			allocated_size,
		} = self.compute_sizes(constraints);

		if self.direction == Axis::Horizontal {
			let size = constraints.constrain(Size(DVec2::new(main_size, cross_size)));
			main_size = size.x;
			cross_size = size.y;
		} else {
			let size = constraints.constrain(Size(DVec2::new(cross_size, main_size)));
			main_size = size.y;
			cross_size = size.x;
		};
		let actual_size_delta = main_size - allocated_size;
		let overflow = 0_f64.max(-actual_size_delta);
		if overflow > 0. {
			info!("Overflow of {overflow}");
		}
		let remaining_space = 0_f64.max(actual_size_delta);
		let flip_main_axis = false;
		let child_count = self.children.len();
		let (leading_space, between_sapce) = match self.main_axis_alignment {
			MainAxisAlignment::Start => (0., 0.),
			MainAxisAlignment::End => (remaining_space, 0.),
			MainAxisAlignment::Center => (remaining_space / 2., 0.),
			MainAxisAlignment::SpaceBetween => (0., if child_count > 1 { remaining_space / (child_count - 1) as f64 } else { 0. }),
			MainAxisAlignment::SpaceAround => {
				let between_space = if child_count > 0 { remaining_space / child_count as f64 } else { 0. };
				(between_space / 2., between_space)
			}
			MainAxisAlignment::SpaceEvenly => {
				let between_space = if child_count > 0 { remaining_space / (child_count + 1) as f64 } else { 0. };
				(between_space, between_space)
			}
		};
		let mut child_main_position = if flip_main_axis { main_size - leading_space } else { leading_space };
		for child in 0..child_count {
			let render_params = self.children.render_params(child);
			let size = render_params.size;
			let child_cross_position = match self.cross_axis_alignment {
				CrossAxisAlignment::Start => 0.,
				CrossAxisAlignment::End => cross_size - size[self.direction.cross()],
				CrossAxisAlignment::Center => cross_size / 2. - size[self.direction.cross()] / 2.,
				CrossAxisAlignment::Stretch => 0.,
			};
			if flip_main_axis {
				child_main_position -= size[self.direction];
			}
			render_params.offset = match self.direction {
				Axis::Horizontal => DVec2::new(child_main_position, child_cross_position),
				Axis::Vertical => DVec2::new(child_cross_position, child_main_position),
			};
			if flip_main_axis {
				child_main_position -= between_sapce;
			} else {
				child_main_position += size[self.direction] + between_sapce;
			}
		}
		match self.direction {
			Axis::Horizontal => Size(DVec2::new(main_size, cross_size)),
			Axis::Vertical => Size(DVec2::new(cross_size, main_size)),
		}
	}

	fn render_params(&mut self) -> &mut RenderParams {
		&mut self.render_params
	}
}

mod tuple_impls {
	use super::{BoxConstraint, RenderParams, Size, UiElement, UiElementList};
	macro_rules! tuple {
		($length:literal: $($x:ident=$index:tt),*) => {
			impl<$($x:UiElement),*> UiElementList for ($($x,)*) {
				fn len(&self) -> usize {
					$length
				}
				fn render_params(&mut self, index: usize) -> &mut RenderParams {
					match index{
						$($index => self.$index.render_params(),)*
						_ => panic!("Index out of range: index={} length={}", index, self.len()),
					}
				}
				fn layout_nth(&mut self, index: usize, _box_constraint: BoxConstraint) -> Size{
					match index{
						$($index => self.$index.layout(_box_constraint),)*
						_ => panic!("Index out of range: index={} length={}", index, self.len()),
					}
				}
			}
		};
	}
	tuple!(0 :);
	tuple!(1 : A=0);
	tuple!(2 : A=0, B=1);
	tuple!(3 : A=0, B=1, C=2);
	tuple!(4 : A=0, B=1, C=2, D=3);
	tuple!(5 : A=0, B=1, C=2, D=3, E=4);
	tuple!(6 : A=0, B=1, C=2, D=3, E=4, F=5);
	tuple!(7 : A=0, B=1, C=2, D=3, E=4, F=5, G=6);
	tuple!(8 : A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7);
	tuple!(9 : A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8);
	tuple!(10: A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8, J=9);
	tuple!(11: A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8, J=9, K=10);
	tuple!(12: A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8, J=9, K=10, L=11);
	tuple!(13: A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8, J=9, K=10, L=11, M=12);
	tuple!(14: A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8, J=9, K=10, L=11, M=12, N=13);
	tuple!(15: A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8, J=9, K=10, L=11, M=12, N=13, O=14);
	tuple!(16: A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7, I=8, J=9, K=10, L=11, M=12, N=13, O=14, P=15);
}
pub use tuple_impls::*;

#[test]
fn my_own_layout() {
	let mut layout = Container {
		child: Flex { children: (), ..Default::default() },
		..default()
	};
	layout.layout(BoxConstraint::loose(DVec2::new(300., 200.)));
}

/// How the children should be placed along the main axis in a flex layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MainAxisAlignment {
	/// Place the children as close to the start of the main axis as possible.
	#[default]
	Start,
	/// Place the children as close to the end of the main axis as possible.
	End,
	/// Place the children as close to the middle of the main axis as possible.
	Center,
	/// Place the free space evenly between the children.
	SpaceBetween,
	/// Place the free space evenly between the children as well as half of that
	/// space before and after the first and last child.
	SpaceAround,
	/// Place the free space evenly between the children as well as before and
	/// after the first and last child.
	SpaceEvenly,
}

/// How the children should be placed along the cross axis in a flex layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CrossAxisAlignment {
	/// Place the children with their start edge aligned with the start side of
	/// the cross axis.
	#[default]
	Start,
	/// Place the children as close to the end of the cross axis as possible.
	End,
	/// Place the children so that their centers align with the middle of the
	/// cross axis.CrossAxisAlignment
	Center,
	/// Force the full width
	Stretch,
}

/// How much space should the main axis take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MainAxisSize {
	Min,
	#[default]
	Max,
}

/// Direction
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Axis {
	#[default]
	Horizontal,
	Vertical,
}

impl Axis {
	pub const fn cross(self) -> Self {
		match self {
			Axis::Horizontal => Axis::Vertical,
			Axis::Vertical => Axis::Horizontal,
		}
	}
}

impl std::ops::Index<Axis> for DVec2 {
	type Output = f64;

	fn index(&self, index: Axis) -> &Self::Output {
		match index {
			Axis::Horizontal => &self.x,
			Axis::Vertical => &self.y,
		}
	}
}
