import os
import numpy

def parse_mat(lines):
	vals = {}
	name = None
	materials = {}
	for line in lines:
		line = line[:-1]
		if len(line) == 0 or line[:1] == '#':
			continue
		
		if line.startswith("newmtl "):
			if name != None:
				materials[name] = vals
			name = line[len("newmtl "):]
			vals = {}
			continue
		split_line = line.split(" ")
		cmd = split_line[0]
		vals[cmd] = [float(x) for x in split_line[1:]]
	if name!=None:
		materials[name] = vals
	return materials

def encode_verts(verts,vert_normal, faces, colour, vert_offset):
	filtered_verts = []
	filtered_faces = []

	for face in faces:
		resolved_face = []
		for [vert_index, _, normal_index] in face:
			filtered_verts.append(verts[vert_index-1] + vert_normal[normal_index-1] + colour)
			resolved_face.append(vert_offset)
			vert_offset+=1
		filtered_faces.append(resolved_face)
	return (numpy.array(filtered_verts, dtype=numpy.single, ndmin=2).tobytes(), numpy.array(filtered_faces, dtype="<u4", ndmin=2).tobytes(), vert_offset)
	
	

def parse_obj(lines, path, file):
	verts = []
	vert_normal = []
	materials = {}
	current_colour = [0,0,0]
	faces = []
	vert_data = bytes()
	index_data = bytes()
	vert_offset=0
	for line in lines:
		line = line[:-1]
		if len(line) == 0 or line[:1] == '#':
			continue
		split_line = line.split(" ")
		cmd = split_line[0]
		if cmd == "mtllib":
			lib_path = os.path.join(path, split_line[1])
			print("Mat at path", lib_path)
			with open(lib_path) as f:
				materials = parse_mat(f.readlines())
		elif cmd == "v":
			[x,y,z] = [float(i) for i in split_line[1:]]
			verts.append([z,x,y])
		elif cmd == "vn":
			[x,y,z] = [float(i) for i in split_line[1:]]
			vert_normal.append([x,y,z])
		elif cmd == "usemtl":
			if len(faces) > 0:
				(encoded_v, encoded_f, vert_offset) = encode_verts(verts,vert_normal, faces, current_colour, vert_offset)
				vert_data += encoded_v
				index_data += encoded_f
				faces = []
			material = ' '.join(split_line[1:])
			current_colour = materials[material]["Kd"]
		elif cmd == "f":
			faces.append([[int(i) for i in i.split("/")] for i in split_line[1:]])
	if len(faces) > 0:
		(encoded_v, encoded_f, vert_offset) = encode_verts(verts,vert_normal, faces, current_colour, vert_offset)
		vert_data += encoded_v
		index_data += encoded_f
		faces = []

	print("offset", vert_offset)
	with open(os.path.join(path, "..", "dat", file[:-3]+"dat"), "wb+") as f:
		b = bytes()
		b += len(vert_data).to_bytes(4, byteorder="little")
		b += len(index_data).to_bytes(4, byteorder="little")
		print( len(vert_data),  len(index_data))
		b += vert_data
		b += index_data
		f.write(b)
		print("written", len(b))


for path, directories, files in os.walk(os.path.join(".", "..", "assets", "obj")):
	for file in files:
		with open(os.path.join(path, file)) as f:
			lines = f.readlines()
			if file.endswith("obj"):
				parse_obj(lines, path, file)

