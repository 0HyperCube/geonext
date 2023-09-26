import math
from typing import Iterator, List, Tuple
from xml.dom import minidom
from dataclasses import dataclass
import copy
import bisect
import numpy as np
# The command characters in svg
svg_commands = "LMHVlmhvZz"

@dataclass
class Point:
	"""An x and y position"""
	x: float
	y: float
	def distance(self, other) -> float:
		return math.sqrt((self.x - other.x) ** 2 + (self.y - other.y) ** 2)
	def __add__(self, other):
		return Point(self.x + other.x, self.y + other.y)
	def __truediv__(self, other):
		return Point(self.x / other, self.y / other)
	
@dataclass(order=True)
class IntPoint:
	"""An x and y position"""
	x: int
	y: int

@dataclass
class Path:
	pos: Point
	name: str


def parse_svg_float(index: int, svg: str) -> Tuple[float, int]:
	"""Parses an svg float (can be ended with a second negitive or a second decimal point or a space or a command). Returns the new index."""

	result = ""

	# Check for an initial negative
	if svg[index] == "-":
		result = "-"
		index += 1

	parsed_decimal_point = False

	while (
		index < len(svg)
		and svg[index] != " "
		and svg[index] != ","
		and not svg[index] in svg_commands
		and svg[index] != "-"
		and (not parsed_decimal_point or svg[index] != ".")
	):
		result += svg[index]
		if svg[index] == ".":
			parsed_decimal_point = True
		index += 1

	while index < len(svg) and svg[index] in ", ":
		index += 1
	return (float(result), index)

def parse_path(svg, name):
	"""Converts an svg to a KDTree and a bounding box"""

	index = 0
	points: list[Point] = []
	current_position = Point(0, 0)
	min_pos = Point(math.inf, math.inf)
	max_pos = Point(-math.inf, -math.inf)
	centre = Point(0, 0)
	centre_points = 0
	calculated_centre = []
	command = "M"

	while index < len(svg):
		# Implicitly use last command if no new command is found
		if svg[index] in svg_commands:
			command = svg[index]
			index += 1

		if command == "L" or command == "M":
			(x, index) = parse_svg_float(index, svg)
			(y, index) = parse_svg_float(index, svg)
			current_position = Point(x, y)

		elif command == "h":
			(v, index) = parse_svg_float(index, svg)
			current_position += Point(v, 0)

		elif command == "v":
			(v, index) = parse_svg_float(index, svg)
			current_position += Point(0, v)

		elif command == "H":
			(v, index) = parse_svg_float(index, svg)
			current_position = Point(v, current_position.y)

		elif command == "V":
			(v, index) = parse_svg_float(index, svg)
			current_position = Point(current_position.x, v)

		elif command == "l" or command == "m":
			(x, index) = parse_svg_float(index, svg)
			(y, index) = parse_svg_float(index, svg)
			
			current_position += Point(x, y)

		elif command == "z" or command == "Z":
			calculated_centre.append((centre_points, centre))
			centre_points = 0
			centre = Point(0, 0)
			continue

		points.append(current_position)
		centre += current_position
		centre_points += 1

		min_pos = Point(
				min(min_pos.x, current_position.x),
				min(min_pos.y, current_position.y)
		)
		max_pos = Point(
				max(max_pos.x, current_position.x),
				max(max_pos.y, current_position.y)
		)
		

	calculated_centre.append((centre_points, centre))
	(num, total) = max(calculated_centre, key=lambda x: x[0])
	centre = total / num

	return Path(min_pos, name)

def y_to_row_int(y:float) -> int:
	"""Convert from svg y value to an integer row"""
	return round(((y - offset_top) / (radius*2)))

def xy_to_col_int(x: float, y: int) -> int:
	"""Convert from svg x value and an integer row to an integer column value"""
	return round((x-offset_left) / (apothem*2) + (y%2)*.5)

def compute_radius(y_positions: List[float]) -> float:
	"""Find the radius of the hexagons in the svg (distance from point to centre)"""
	y_positions = copy.deepcopy(y_positions)
	y_positions.sort()
	delta_y_positions = [y_positions[index + 1] - y_positions[index] for index in range(len(y_positions) - 1) if y_positions[index + 1] - y_positions[index] > 2 and y_positions[index + 1] - y_positions[index] < 4]
	diameter = max(delta_y_positions)
	return diameter / 2

def compute_apothem() -> float:
	"""Computes the apothem of the svg hexagons (distance from centre to edge)"""
	rows = [[] for _i in range(max_y+1)]
	for value in path_strings:
		y = value.pos.y
		rows[y_to_row_int(y)].append(value)

	count = 0
	total = 0
	for row in rows:
		row.sort(key=lambda value: value.pos.x)
		x_pos = [value.pos.x for value in row]
		
		for i in range(len(x_pos) - 1):
			delta_x = (x_pos[i+1] - x_pos[i])
			if delta_x > 1 and delta_x < 4:
				total += delta_x / 2
				count += 1
	return total / count

def offset_x(x: int, y: int) -> float:
	"""Converts the integer column value to a float, shifting by a half if necessary"""
	return x - (y%2)*.5

def to_axial(x: int, y: int) -> Tuple[int, int]:
	q = x - (y + (y&1)) / 2
	r = y
	if round(q) != q:
		raise TypeError()
	return (int(q), r)

def from_axial(axial: Tuple[int, int]) -> Tuple[int, int]:
	(q,r) = axial
	col = q + (r + (r&1)) / 2
	row = r
	return col, row

def neighbours(axial: Tuple[int, int]) -> Iterator[Tuple[int, int]]:
	for (offset_q, offset_r) in [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)]:
		yield (axial[0] + offset_q, axial[1] + offset_r)

def plot(normalised_x, normalised_y, max_x, max_y, colours):
	"""Plot the map in matplotlib"""
	import matplotlib.pyplot as plt

	MAT_TO_PX = 1/plt.rcParams['figure.dpi']
	MAP_SCALE = 5
	_fig, ax = plt.subplots(figsize=(max_x*MAT_TO_PX*apothem*MAP_SCALE, max_y*MAT_TO_PX*radius*MAP_SCALE))
	
	
	ax.scatter([offset_x(val, normalised_y[index]) for index, val in enumerate(normalised_x)],[max_y-y for y in normalised_y], marker="h", s= (radius*2*MAP_SCALE), c=colours)

	plt.show()

def write_file(np_array):
	with open("../assets/map.txt", "wb") as f:
		f.write(np_array)

from pyproj import CRS, Transformer

crs_4326 = CRS.from_epsg(4326) # north then east - crs is used by nasa
long_lat1 = CRS.from_proj4("+proj=latlon")
robin = CRS.from_proj4("+proj=robin +R=1000") # East then north - robin is used by mapchart
long_lat2 = CRS.from_proj4("+proj=latlon +R=1000")

lat_long_to_nasa = Transformer.from_crs(long_lat1, crs_4326, always_xy=True)
nasa_to_lat_long = Transformer.from_crs(crs_4326, long_lat1, always_xy=True)
lat_long_to_robin = Transformer.from_crs(long_lat2, robin, always_xy=True)
robin_to_lat_long = Transformer.from_crs(robin, long_lat2, always_xy=True)


(nasa_left, nasa_bottom, nasa_right, nasa_top) = lat_long_to_nasa.transform_bounds(-90, -90, 90, 90)
(robin_left, robin_bottom, robin_right, robin_top) = lat_long_to_robin.transform_bounds(-90, -90, 90, 90)

ranges = [math.inf, math.inf, -math.inf, -math.inf]
class ImageSampler:
	"""An image sampler allows the pixel under the hexagon to be determined"""
	image: np.ndarray
	bounds_min: Point
	size: Point
	def get_pixel(self, coord: Tuple[int, int]) -> int:
		return self.image[min(self.image.shape[0] - 1, coord[1])][min(self.image.shape[1] - 1, coord[0])]
	def get_hex(self, x: int, y:int) -> Tuple[int, int] | None:
		
		global ranges
		svg_units = Point(offset_x(x, y)  , float(y) )
		
		ranges = [min(ranges[0], svg_units.x),min(ranges[1], svg_units.y) ,max(ranges[2], svg_units.x), max(ranges[3], svg_units.y)]
		
		normalised_svg = Point(svg_units.x / WIDTH, svg_units.y / HEIGHT)
		normalised_svg = Point((normalised_svg.x+0.032869) / 0.838871839, (normalised_svg.y-0.022986417) / 1.103373425)
		normalised_svg = Point(min(1, max(0,normalised_svg.x)),min(1, max(0,normalised_svg.y)))
		
		robin_units = Point(robin_left + normalised_svg.x * (robin_right - robin_left) / 1.059369064, robin_bottom + normalised_svg.y * (robin_top - robin_bottom))
		robin_units = Point(robin_units.x, robin_units.y)
		
		lat_long = robin_to_lat_long.transform(robin_units.x, robin_units.y)
		if lat_long[0] < -90 or lat_long[0] > 90 or lat_long[1] < -90 or lat_long[1] > 90:
			return None
		
		nasa = lat_long_to_nasa.transform(lat_long[0],lat_long[1])
		
		normalised_pos = Point((nasa[0] - nasa_left) / (nasa_right - nasa_left), (nasa[1] - nasa_bottom) / (nasa_top - nasa_bottom))

		x = min(self.image.shape[1],max(0, int(normalised_pos.x * self.image.shape[1])))
		y = min(self.image.shape[0],max(0 , int(normalised_pos.y * self.image.shape[0])))
		return (min(self.image.shape[1],max(0, x)), min(self.image.shape[0],max(0, y)))

	
	def get_colour(self, x: int, y:int) -> Tuple[float, float, float]:
		pos = self.get_hex(x,y)
		val = 1 if pos == None else self.get_pixel(pos) / 255
		return (1, val, 1)
	
	def __init__(self, file_name: str, normalised_x: List[int], normalised_y: List[int]):
		import imageio.v3 as iio

		self.bounds_min = Point(min(normalised_x), min(normalised_y))
		self.bounds_min = Point(0,0)
		self.size = Point((max(normalised_x) - self.bounds_min.x)*.99, (max(normalised_y) - self.bounds_min.x) *1.25)
		self.image = iio.imread(file_name)



radius = 1.5
apothem = radius * math.cos(math.radians(180/6))

WIDTH = 930 / (radius*2)
HEIGHT = 452.315 / (radius*2)

with open("map_base.svg") as svg_file:
	# Parse the xml
	doc = minidom.parse(svg_file)
	path_strings = [parse_path(path.getAttribute('d'), path.getAttribute('id')) for path
					in doc.getElementsByTagName('path') if path.getAttribute("id")!="State_borders"]
	# Don't leak memory
	doc.unlink()

	# Extract x and y positions for each hex
	y_positions = [value.pos.y for value in path_strings]
	radius = compute_radius(y_positions)

	offset_top, offset_left = 0,0
	normalised_y = [y_to_row_int(y) for y in y_positions]
	max_y = max(normalised_y)

	apothem = compute_apothem()

	x_positions = [value.pos.x for value in path_strings]
	normalised_x = [xy_to_col_int(x_positions[index], normalised_y[index]) for index in range(len(path_strings))]
	max_x = max(normalised_x)

	# Get country names for each hex, and create a mapping from u8 -> country name
	names = list(map(lambda path: path.name[:path.name.rfind('_')], path_strings))
	names_register = list(set(names))
	names_register.sort()

	# Convert a country name to the index used in the mapping
	def compute_index(name):
		index = bisect.bisect_left(names_register, name)
		if names_register[index] == name:
			return index
		print("Not found")
		return 254

	# Store (country name index, x, y) in a sorted list
	section = list(zip(map(compute_index, names), normalised_x, normalised_y))
	section.sort(key = lambda s: s[1]*max_y+s[2], reverse=True)

	# Export axial coordinate files
	axial_coords = list(map(to_axial, normalised_x, normalised_y))
	hex_names = list(map(lambda path: path.name, path_strings))
	axial_hex_lookup = dict(zip(axial_coords, hex_names))
	neighbour_lookup = dict(zip(hex_names, map(lambda coord: list(map(axial_hex_lookup.__getitem__, filter(axial_hex_lookup.__contains__, neighbours(coord)))), axial_coords)))
	print(neighbour_lookup)
	import json
	axial_json = json.dumps(dict(map(lambda b: (str(b[0]), b[1]), axial_hex_lookup.items())), sort_keys=True, indent=4)
	with open("../assets/axial-coordinates.json", "w+") as f:
		f.write(axial_json)
	neighbours_json = json.dumps(neighbour_lookup, sort_keys=True, indent=4)
	with open("../assets/neighbours.json", "w+") as f:
		f.write(neighbours_json)


	# Load nasa data maps
	snow = ImageSampler('MOD10C1_M_SNOW_2022-02_snow.PNG', normalised_x, normalised_y)
	vegitation = ImageSampler('MOD_NDVI_M_2023-01_vegitation.PNG', normalised_x, normalised_y)
	topo = ImageSampler('SRTM_RAMP2_TOPO_2000.PNG', normalised_x, normalised_y)

	width = max_x + 1
	height = max_y + 1

	# Matplotlib for testing
	#normalised_x = [x % width for x in range(width * height)]
	#normalised_y = [x // width for x in range(width * height)]
	#c = [topo.get_colour(normalised_x[index], normalised_y[index]) for index in range(len(normalised_y))]
	#plot(normalised_x, normalised_y, max_x, max_y, "blue")
	
	channels = 4

	# Header info, u16 for width, u16 for height, u16 for channel count
	a = bytearray(width.to_bytes(2, byteorder="little") + height.to_bytes(2, byteorder="little") + channels.to_bytes(2, byteorder="little"))
	
	# u8 for number of country names
	a += len(names_register).to_bytes(1, byteorder="little")
	# Encode the utf8 of the country names, starting each string with a u8 for the length
	for name in names_register:
		name_bytes = bytes(name, encoding="utf8")
		a += len(name_bytes).to_bytes(1, byteorder="little")
		a += name_bytes

	# Encode the country name index, snow and vegitation for each hex, top to bottom, left to right
	data_start = len(a)
	for x in range(0, max_x+1):
		for y in range(0,max_y+1):
			next_section = len(section) == 0 or section[len(section)-1]
			if next_section != True and next_section[1] == x and next_section[2] == y:
				section.pop()
				a.append(next_section[0])
			else:
				a.append(254)

			pos = snow.get_hex(x,y)
			
			if pos == None:
				a.append(255)
				a.append(255)
				a.append(255)
			else:
				a.append(snow.get_pixel(pos))
				a.append(vegitation.get_pixel(pos))
				a.append(topo.get_pixel(pos))



	write_file(a)
