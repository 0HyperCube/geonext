use glam::UVec2;

/// Allocates rects in an atlas
#[derive(Debug, Default)]
pub struct Atlas {
	free_spaces: Vec<[UVec2; 2]>,
}

impl Atlas {
	/// Creates a new atlas allocator based on a size
	pub fn new(size: UVec2) -> Self {
		let free_spaces = vec![[UVec2::ZERO, size]];
		Self { free_spaces }
	}

	/// Allocates a new rect in the texture map, returning the position of the rect if found.
	/// Based upon https://github.com/TeamHypersomnia/rectpack2D#algorithm
	pub fn allocate_rect(&mut self, size: UVec2) -> Option<UVec2> {
		if let Some((index, &[pos, container_size])) = self.free_spaces.iter().enumerate().rev().find(|(_, [_, container])| container.cmpge(size).all()) {
			// Remove the chosen free space
			self.free_spaces.swap_remove(index);

			match (container_size.x - size.x, container_size.y - size.y) {
				(0, 0) => {}
				(spare_width, 0) => self.free_spaces.push([UVec2::new(pos.x + size.x, pos.y), UVec2::new(spare_width, container_size.y)]),
				(0, spare_height) => self.free_spaces.push([UVec2::new(pos.x, pos.y + size.y), UVec2::new(container_size.x, spare_height)]),
				(spare_width, spare_height) => {
					if spare_width > spare_height {
						self.free_spaces.push([UVec2::new(pos.x + size.x, pos.y), UVec2::new(spare_width, container_size.y)]);
						self.free_spaces.push([UVec2::new(pos.x, pos.y + size.y), UVec2::new(size.x, spare_height)]);
					} else {
						self.free_spaces.push([UVec2::new(pos.x, pos.y + size.y), UVec2::new(container_size.x, spare_height)]);
						self.free_spaces.push([UVec2::new(pos.x + size.x, pos.y), UVec2::new(spare_width, size.y)]);
					}
				}
			}
			Some(pos)
		} else {
			None
		}
	}
}
