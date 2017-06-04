# `nom-obj` 
[![Build Status](https://travis-ci.org/dwerner/nom-obj.svg?branch=master)](https://travis-ci.org/dwerner/nom-obj)
[![Crates.io Version](https://img.shields.io/crates/v/nom-obj.svg)](https://crates.io/crates/nom-obj)

An obj/mtl (wavefront 3d model format) file format parser written with nom.

[https://en.wikipedia.org/wiki/Wavefront_.obj_file](https://en.wikipedia.org/wiki/Wavefront_.obj_file)

This crate was designed to parse an obj, and any referenced mtl files that it points to. It doesn't try to implement the entire spec, but instead just relies on triangulated meshes. The purpose was to provide a model parser for [sg-engine](https://github.com/dwerner/sg-engine). Since the purpose was to generate a structure of data for the GPU to consume, I went with the opinionated stance of interleaved vertex data (vertex/texture/normal information). Both glutin and vulkano require implementing a trait on a vertex (either using a macro or manually), so we have to copy the vertex data into our desired format.


```rust
let obj = Obj::create("assets/cube.obj");

// Multiple mesh objects are supported, stored in objects[] vec
let Interleaved{ v_vt_vn, idx } = obj.objects[0].interleaved();

// Copy interleaved vertex information
let verts = v_vt_vn.iter()
	.map(|&(v,vt,vn)| Vertex::create(v.0, v.1, v.2, vt.0, vt.1, vt.2, vn.0, vn.1, vn.0) )
	.collect::<Vec<_>>();

assert!(verts.len() > 0);

let indices = idx.iter()
	.map(|x:&usize| *x as u16)
	.collect::<Vec<_>>();

use std::path::Path;
let path_str = obj.get_mtl().diffuse_map.clone();
let material_path = Path::new(&path_str);
let diffuse_map = image::open(material_path).expect("unable to open image file from material");

```

Current limitations:
- only supports a single diffuse texture, (though most of the wiring is in place to support others)
- file paths for materials need som  attention in Windows.

Todo:
- some usage examples are needed. For now it can be seen in [sg-engine](https://github.com/dwerner/sg-engine/blob/master/game_state/src/model.rs) 
- Support multiple material per mtl file - multiple materials can be defined in an mtl file.
- fix paths to assets (probably only works on unix currently)

Notes:
- obj and mtl parsers are implemented in terms of nom parser combinators, and while the learning curve was a bit high, I really enjoyed writing parsers this way. It's a very different experience from classical manual parsing. obj and mtl are plain text formats. Thanks Geal, nom is an excellent tool. Be sure to check out [nom](https://github.com/geal/nom) for your parsing needs in Rust.
