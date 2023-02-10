from xml.dom import minidom
# The command characters in svg
svg_commands = "LMHVlmhvZz"


def parse_svg_float(index, svg):
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
		and not svg[index] in svg_commands
		and svg[index] != "-"
		and (not parsed_decimal_point or svg[index] != ".")
	):
		result += svg[index]
		if svg[index] == ".":
			parsed_decimal_point = True
		index += 1

	while index < len(svg) and svg[index] == " ":
		index += 1
	return (float(result), index)
def parse_path(svg, name):
	"""Converts an svg to a KDTree and a bounding box"""

	index = 0
	points = []
	current_position = (0, 0)
	#bounding_box = (inf, inf, -inf, -inf)
	centre = (0, 0)
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
			current_position = (x, y)

		elif command == "h":
			(v, index) = parse_svg_float(index, svg)
			current_position = (current_position[0] + v, current_position[1])

		elif command == "v":
			(v, index) = parse_svg_float(index, svg)
			current_position = (current_position[0], current_position[1] + v)

		elif command == "H":
			(v, index) = parse_svg_float(index, svg)
			current_position = (v, current_position[1])

		elif command == "V":
			(v, index) = parse_svg_float(index, svg)
			current_position = (current_position[0], v)

		elif command == "l" or command == "m":
			(x, index) = parse_svg_float(index, svg)
			(y, index) = parse_svg_float(index, svg)
			current_position = (current_position[0] + x, current_position[1] + y)

		elif command == "z" or command == "Z":
			calculated_centre.append((centre_points, centre))
			centre_points = 0
			centre = (0, 0)
			continue

		points.append(current_position)
		centre = (centre[0] + current_position[0], centre[1] + current_position[1])
		centre_points += 1

		# bounding_box = (
		# 	min(bounding_box[0], current_position[0]),
		# 	min(bounding_box[1], current_position[1]),
		# 	max(bounding_box[2], current_position[0]),
		# 	max(bounding_box[3], current_position[1]),
		# )

	calculated_centre.append((centre_points, centre))
	(num, total) = max(calculated_centre, key=lambda x: x[0])
	centre = [total[0] / num, total[1] / num]

	print(">>>",len(points), points, svg)

	return points, name


with open("map_base.svg") as svg_file:
	# Parse the xml
	doc = minidom.parse(svg_file)
	path_strings = [parse_path(path.getAttribute('d'), path.getAttribute('id')) for path
					in doc.getElementsByTagName('path') if path.getAttribute("id")!="State_borders"]
	# Don't leak memory
	doc.unlink()

	print(len(path_strings))
