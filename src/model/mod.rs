use parser::obj::{FaceIndex, ObjLine, ObjParser};

use parser::mtl::{MtlLine, MtlParser};

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

type T3<T> = (T, T, T);
type T4<T> = (T, T, T, T);

pub struct Obj {
    pub comments: Vec<String>,
    pub objects: Vec<ObjObject>,
}
impl Obj {
    pub fn read_file(filename: &str) -> Result<Self, Box<dyn Error>> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader, Some(filename))
    }

    pub fn from_reader<T>(
        reader: BufReader<T>,
        mtl_search_path: Option<&str>,
    ) -> Result<Self, Box<dyn Error>>
    where
        T: std::io::Read,
    {
        let parser = ObjParser::new(reader);

        let mut comments = Vec::new();
        let mut objects = Vec::new();
        let mut object = ObjObject::new();

        for line in parser {
            match line {
                ObjLine::ObjectName(name) => {
                    // new object encountered, when multiple objects exist
                    if object.name.is_some() {
                        objects.push(object);
                        object = ObjObject::new();
                    }
                    object.name = Some(name);
                }
                ObjLine::MtlLib(name) => {
                    if let Some(mtl_search_path) = mtl_search_path {
                        if let Some(parent) = Path::new(mtl_search_path).parent() {
                            let mtl_path = Path::join(parent, name);
                            let file = File::open(mtl_path)?;
                            let reader = BufReader::new(file);
                            let mtl_parser = MtlParser::new(reader);
                            for line in mtl_parser {
                                if let MtlLine::DiffuseMap(diffuse_map) = line {
                                    let path = Path::join(parent, diffuse_map);
                                    if let Some(diffuse) = path.to_owned().to_str() {
                                        let diffuse_map = diffuse.to_string();
                                        object.material = Some(ObjMaterial { diffuse_map });
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                ObjLine::Vertex(..) => object.vertices.push(line),
                ObjLine::VertexParam(..) => object.vertex_params.push(line),
                ObjLine::Face(..) => object.faces.push(line),
                ObjLine::Normal(..) => object.normals.push(line),
                ObjLine::TextureUVW(..) => object.texture_coords.push(line),
                ObjLine::Comment(comment) => comments.push(comment),
                _ => {}
            }
        }
        objects.push(object);
        Ok(Obj { comments, objects })
    }
}

#[derive(Debug, Default)]
pub struct ObjObject {
    pub name: Option<String>,
    pub material: Option<ObjMaterial>,
    pub vertices: Vec<ObjLine>,
    pub normals: Vec<ObjLine>,
    pub texture_coords: Vec<ObjLine>,
    pub vertex_params: Vec<ObjLine>,
    pub faces: Vec<ObjLine>,
}

#[derive(Debug)]
pub struct ObjMaterial {
    pub diffuse_map: String,
}

impl ObjObject {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn vertices(&self) -> &Vec<ObjLine> {
        &self.vertices
    }
    pub fn vertex_params(&self) -> &Vec<ObjLine> {
        &self.vertex_params
    }
    pub fn normals(&self) -> &Vec<ObjLine> {
        &self.normals
    }
    pub fn texture_coords(&self) -> &Vec<ObjLine> {
        &self.texture_coords
    }

    #[inline]
    fn get_v_tuple(&self, face_index: &FaceIndex) -> (f32, f32, f32, f32) {
        let &FaceIndex(ix, _, _) = face_index;
        match self.vertices[(ix as usize) - 1] {
            ObjLine::Vertex(x, y, z, w) => (x, y, z, w.unwrap_or(1.0)),
            _ => panic!("not a vertex"),
        }
    }

    #[inline]
    fn get_vt_tuple(&self, face_index: &FaceIndex) -> (f32, f32, f32) {
        let &FaceIndex(_, vt, _) = face_index;
        if let Some(vt) = vt {
            match self.texture_coords[(vt as usize) - 1] {
                ObjLine::TextureUVW(u, v, w) => (u, v, w.unwrap_or(0.0)),
                _ => panic!("not a vertex"),
            }
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    #[inline]
    fn get_vn_tuple(&self, face_index: &FaceIndex) -> (f32, f32, f32) {
        let &FaceIndex(_, _, vn) = face_index;
        if let Some(vn) = vn {
            match self.normals[(vn as usize) - 1] {
                ObjLine::Normal(x, y, z) => (x, y, z),
                _ => panic!("not a vertex"),
            }
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    #[inline]
    fn interleave_tuples(&self, id: &FaceIndex) -> (T4<f32>, T3<f32>, T3<f32>) {
        let vert = self.get_v_tuple(id);
        let text = self.get_vt_tuple(id);
        let norm = self.get_vn_tuple(id);
        (vert, text, norm)
    }

    pub fn interleaved(&self) -> Interleaved {
        let mut vertex_map = HashMap::new();

        let mut data = Interleaved {
            v_vt_vn: Vec::new(),
            idx: Vec::new(),
        };

        for i in 0usize..self.faces.len() {
            match self.faces[i] {
                ObjLine::Face(ref id1, ref id2, ref id3) => {
                    let next_idx = (id1.0 as usize) - 1;
                    data.idx.push(next_idx);
                    vertex_map
                        .entry(next_idx)
                        .or_insert_with(|| self.interleave_tuples(id1));

                    let next_idx = (id2.0 as usize) - 1;
                    data.idx.push(next_idx);
                    vertex_map
                        .entry(next_idx)
                        .or_insert_with(|| self.interleave_tuples(id2));

                    let next_idx = (id3.0 as usize) - 1;
                    data.idx.push(next_idx);
                    vertex_map
                        .entry(next_idx)
                        .or_insert_with(|| self.interleave_tuples(id3));
                }
                _ => panic!("Found something other than a ObjLine::Face in object.faces"),
            }
        }
        for i in 0usize..vertex_map.len() {
            data.v_vt_vn.push(vertex_map.remove(&i).unwrap());
        }
        data
    }
}

pub struct Interleaved {
    pub v_vt_vn: Vec<(T4<f32>, T3<f32>, T3<f32>)>,
    pub idx: Vec<usize>,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn cube_format_interleaved() -> Result<(), Box<dyn Error>> {
        let o = Obj::read_file("assets/cube.obj")?;
        let interleaved = o.objects[0].interleaved();
        println!("{:?}", o.objects[0].faces);
        assert_eq!(o.objects[0].faces.len(), 12);
        assert_eq!(interleaved.v_vt_vn.len(), 8);

        assert!(o.objects[0].material.is_some());
        let ObjMaterial { diffuse_map } = o.objects[0].material.as_ref().unwrap();
        assert_eq!(diffuse_map, "assets/diffuse_map.png");
        Ok(())
    }

    #[test]
    fn negative_texcoord_plane_regression() -> Result<(), Box<dyn Error>> {
        use std::io::Cursor;
        let plane_lines = "mtllib untitled.mtl
o Plane
v -1.000000 0.000000 1.000000
v 1.000000 0.000000 1.000000
v -1.000000 0.000000 -1.000000
v 1.000000 0.000000 -1.000000
vt 1.000000 0.000000
vt 0.000000 1.000000
vt 0.000000 0.000000
vt 1.000000 1.000000
vn 0.0000 1.0000 0.0000
# usemtl None
s off
f 2/1/1 3/2/1 1/3/1
f 2/1/1 4/4/1 3/2/1";

        let cursor = Cursor::new(plane_lines);
        let o = Obj::from_reader(BufReader::new(cursor), None)?;
        let interleaved = o.objects[0].interleaved();

        assert_eq!(o.objects[0].faces.len(), 2);
        assert_eq!(interleaved.v_vt_vn.len(), 4);

        for (v, vt, vn) in interleaved.v_vt_vn {
            assert!(vt.0 >= 0.0);
            assert!(vt.1 >= 0.0);
        }

        Ok(())
    }

    #[test]
    fn cube_obj_has_12_faces() -> Result<(), Box<dyn Error>> {
        // Triangulated model, 12/2 = 6 quads
        let Obj {
            objects: cube_objects,
            ..
        } = Obj::read_file("assets/cube.obj")?;
        assert_eq!(cube_objects[0].faces.len(), 12);
        Ok(())
    }

    #[test]
    fn cube_obj_has_8_verts() -> Result<(), Box<dyn Error>> {
        let o = Obj::read_file("assets/cube.obj")?;
        assert_eq!(o.objects[0].vertices.len(), 8);
        Ok(())
    }

    #[test]
    fn cube_obj_has_1_object() -> Result<(), Box<dyn Error>> {
        let o = Obj::read_file("assets/cube.obj")?;
        assert_eq!(o.objects.len(), 1);
        Ok(())
    }

    #[test]
    fn parses_separate_objects() -> Result<(), Box<dyn Error>> {
        let o = Obj::read_file("assets/four_blue_cubes.obj")?;
        assert_eq!(o.objects.len(), 4);
        Ok(())
    }
}
