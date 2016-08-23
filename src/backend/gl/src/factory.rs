// Copyright 2015 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::rc::Rc;
use std::slice;

use {gl, tex};
use gfx_core as d;
use gfx_core::factory as f;
use gfx_core::factory::Typed;
use gfx_core::format::ChannelType;
use gfx_core::handle;
use gfx_core::handle::Producer;
use gfx_core::mapping::Builder;
use gfx_core::target::{Layer, Level};
use gfx_core::tex as t;

use command::{CommandBuffer, COLOR_DEFAULT};
use {Resources as R, Share, OutputMerger};
use {Buffer, BufferElement, FatSampler, NewTexture,
     PipelineState, ResourceView, TargetView};


fn role_to_target(role: f::BufferRole) -> gl::types::GLenum {
    match role {
        f::BufferRole::Vertex  => gl::ARRAY_BUFFER,
        f::BufferRole::Index   => gl::ELEMENT_ARRAY_BUFFER,
        f::BufferRole::Uniform => gl::UNIFORM_BUFFER,
    }
}

pub fn update_sub_buffer(gl: &gl::Gl, buffer: Buffer, address: *const u8,
                         size: usize, offset: usize, role: f::BufferRole) {
    let target = role_to_target(role);
    unsafe {
        gl.BindBuffer(target, buffer);
        gl.BufferSubData(target,
            offset as gl::types::GLintptr,
            size as gl::types::GLsizeiptr,
            address as *const gl::types::GLvoid
        );
    }
}


/// GL resource factory.
pub struct Factory {
    share: Rc<Share>,
    frame_handles: handle::Manager<R>,
}

impl Clone for Factory {
    fn clone(&self) -> Factory {
        Factory::new(self.share.clone())
    }
}

impl Factory {
    /// Create a new `Factory`.
    pub fn new(share: Rc<Share>) -> Factory {
        Factory {
            share: share,
            frame_handles: handle::Manager::new(),
        }
    }

    pub fn create_command_buffer(&mut self) -> CommandBuffer {
        CommandBuffer::new(self.create_fbo_internal())
    }

    fn create_fbo_internal(&mut self) -> gl::types::GLuint {
        let gl = &self.share.context;
        let mut name = 0 as ::FrameBuffer;
        unsafe {
            gl.GenFramebuffers(1, &mut name);
        }
        info!("\tCreated frame buffer {}", name);
        name
    }

    fn create_buffer_internal(&mut self) -> Buffer {
        let gl = &self.share.context;
        let mut name = 0 as Buffer;
        unsafe {
            gl.GenBuffers(1, &mut name);
        }
        info!("\tCreated buffer {}", name);
        name
    }

    fn init_buffer(&mut self, buffer: Buffer, info: &f::BufferInfo) {
        let gl = &self.share.context;
        let target = role_to_target(info.role);
        if self.share.private_caps.buffer_storage_supported {
            let usage = match info.usage {
                f::Usage::GpuOnly    => 0,
                _                    => gl::MAP_WRITE_BIT | gl::DYNAMIC_STORAGE_BIT,
            };
            unsafe {
                gl.BindBuffer(target, buffer);
                gl.BufferStorage(target,
                    info.size as gl::types::GLsizeiptr,
                    0 as *const gl::types::GLvoid,
                    usage
                );
            }
        }
        else {
            let usage = match info.usage {
                f::Usage::GpuOnly    => gl::STATIC_DRAW,
                f::Usage::Const      => gl::STATIC_DRAW,
                f::Usage::Dynamic    => gl::DYNAMIC_DRAW,
                f::Usage::CpuOnly(_) => gl::STREAM_DRAW,
            };
            unsafe {
                gl.BindBuffer(target, buffer);
                gl.BufferData(target,
                    info.size as gl::types::GLsizeiptr,
                    0 as *const gl::types::GLvoid,
                    usage
                );
            }
        }
    }

    fn create_program_raw(&mut self, shader_set: &d::ShaderSet<R>)
                              -> Result<(gl::types::GLuint, d::shade::ProgramInfo), d::shade::CreateProgramError> {
        use shade::create_program;
        let frame_handles = &mut self.frame_handles;
        let mut shaders = [0; 5];
        let usage = shader_set.get_usage();
        let shader_slice = match shader_set {
            &d::ShaderSet::Simple(ref vs, ref ps) => {
                shaders[0] = *vs.reference(frame_handles);
                shaders[1] = *ps.reference(frame_handles);
                &shaders[..2]
            },
            &d::ShaderSet::Geometry(ref vs, ref gs, ref ps) => {
                shaders[0] = *vs.reference(frame_handles);
                shaders[1] = *gs.reference(frame_handles);
                shaders[2] = *ps.reference(frame_handles);
                &shaders[..3]
            },
            &d::ShaderSet::Tessellated(ref vs, ref hs, ref ds, ref ps) => {
                shaders[0] = *vs.reference(frame_handles);
                shaders[1] = *hs.reference(frame_handles);
                shaders[2] = *ds.reference(frame_handles);
                shaders[3] = *ps.reference(frame_handles);
                &shaders[..4]
            },
        };
        create_program(&self.share.context, &self.share.capabilities,
                       &self.share.private_caps, shader_slice, usage)
    }

    fn view_texture_as_target(&mut self, htex: &handle::RawTexture<R>, level: Level, layer: Option<Layer>)
                              -> Result<TargetView, f::TargetViewError> {
        match (self.frame_handles.ref_texture(htex), layer) {
            (&NewTexture::Surface(_), Some(_)) => Err(f::TargetViewError::Unsupported),
            (&NewTexture::Surface(_), None) if level != 0 => Err(f::TargetViewError::Unsupported),
            (&NewTexture::Surface(s), None) => Ok(TargetView::Surface(s)),
            (&NewTexture::Texture(t), Some(l)) => Ok(TargetView::TextureLayer(t, level, l)),
            (&NewTexture::Texture(t), None) => Ok(TargetView::Texture(t, level)),
        }
    }
}


#[derive(Copy, Clone)]
pub struct RawMapping {
    pointer: *mut ::std::os::raw::c_void,
    target: gl::types::GLenum,
}

impl d::mapping::Raw for RawMapping {
    unsafe fn set<T>(&self, index: usize, val: T) {
        *(self.pointer as *mut T).offset(index as isize) = val;
    }

    unsafe fn to_slice<T>(&self, len: usize) -> &[T] {
        slice::from_raw_parts(self.pointer as *const T, len)
    }

    unsafe fn to_mut_slice<T>(&self, len: usize) -> &mut [T] {
        slice::from_raw_parts_mut(self.pointer as *mut T, len)
    }
}


impl d::Factory<R> for Factory {
    type Mapper = RawMapping;

    fn get_capabilities(&self) -> &d::Capabilities {
        &self.share.capabilities
    }

    fn create_buffer_raw(&mut self, info: f::BufferInfo) -> Result<handle::RawBuffer<R>, f::BufferError> {
        if !self.share.capabilities.constant_buffer_supported && info.role == f::BufferRole::Uniform {
            error!("Constant buffers are not supported by this GL version");
            return Err(f::BufferError::Other);
        }
        let name = self.create_buffer_internal();
        self.init_buffer(name, &info);
        Ok(self.share.handles.borrow_mut().make_buffer(name, info))
    }

    fn create_buffer_const_raw(&mut self, data: &[u8], stride: usize, role: f::BufferRole, bind: f::Bind)
                               -> Result<handle::RawBuffer<R>, f::BufferError> {
        let name = self.create_buffer_internal();
        let info = f::BufferInfo {
            role: role,
            usage: f::Usage::Const,
            bind: bind,
            size: data.len(),
            stride: stride,
        };
        self.init_buffer(name, &info);
        update_sub_buffer(&self.share.context, name, data.as_ptr(), data.len(), 0, role);
        Ok(self.share.handles.borrow_mut().make_buffer(name, info))
    }

    fn create_shader(&mut self, stage: d::shade::Stage, code: &[u8])
                     -> Result<handle::Shader<R>, d::shade::CreateShaderError> {
        ::shade::create_shader(&self.share.context, stage, code)
                .map(|sh| self.share.handles.borrow_mut().make_shader(sh))
    }

    fn create_program(&mut self, shader_set: &d::ShaderSet<R>)
                      -> Result<handle::Program<R>, d::shade::CreateProgramError> {
        self.create_program_raw(shader_set)
            .map(|(name, info)| self.share.handles.borrow_mut().make_program(name, info))
    }

    fn create_pipeline_state_raw(&mut self, program: &handle::Program<R>, desc: &d::pso::Descriptor)
                                 -> Result<handle::RawPipelineState<R>, d::pso::CreationError> {
        use gfx_core::state as s;
        let mut output = OutputMerger {
            draw_mask: 0,
            stencil: match desc.depth_stencil {
                Some((_, t)) if t.front.is_some() || t.back.is_some() => Some(s::Stencil {
                    front: t.front.unwrap_or_default(),
                    back: t.back.unwrap_or_default(),
                }),
                _ => None,
            },
            depth: desc.depth_stencil.and_then(|(_, t)| t.depth),
            colors: [COLOR_DEFAULT; d::MAX_COLOR_TARGETS],
        };
        for i in 0 .. d::MAX_COLOR_TARGETS {
            if let Some((_, ref bi)) = desc.color_targets[i] {
                output.draw_mask |= 1<<i;
                output.colors[i].mask = bi.mask;
                if bi.color.is_some() || bi.alpha.is_some() {
                    output.colors[i].blend = Some(s::Blend {
                        color: bi.color.unwrap_or_default(),
                        alpha: bi.alpha.unwrap_or_default(),
                    });
                }
            }
        }
        let mut inputs = [None; d::MAX_VERTEX_ATTRIBUTES];
        for i in 0 .. d::MAX_VERTEX_ATTRIBUTES {
            inputs[i] = desc.attributes[i].map(|at| BufferElement {
                desc: desc.vertex_buffers[at.0 as usize].unwrap(),
                elem: at.1,
            });
        }
        let pso = PipelineState {
            program: *self.frame_handles.ref_program(program),
            primitive: desc.primitive,
            input: inputs,
            scissor: desc.scissor,
            rasterizer: desc.rasterizer,
            output: output,
        };
        Ok(self.share.handles.borrow_mut().make_pso(pso, program))
    }

    fn create_texture_raw(&mut self, desc: t::Descriptor, hint: Option<ChannelType>, data_opt: Option<&[&[u8]]>)
                          -> Result<handle::RawTexture<R>, t::Error> {
        use gfx_core::tex::Error;
        let caps = &self.share.private_caps;
        if desc.levels == 0 {
            return Err(Error::Size(0))
        }
        let dim = desc.kind.get_dimensions();
        let max_size = self.share.capabilities.max_texture_size;
        if dim.0 as usize > max_size {
            return Err(Error::Size(dim.0));
        }
        if dim.1 as usize > max_size {
            return Err(Error::Size(dim.1));
        }
        let cty = hint.unwrap_or(ChannelType::Uint); //careful here
        let gl = &self.share.context;
        let object = if desc.bind.intersects(f::SHADER_RESOURCE | f::UNORDERED_ACCESS) || data_opt.is_some() {
            let name = if caps.immutable_storage_supported {
                try!(tex::make_with_storage(gl, &desc, cty))
            } else {
                try!(tex::make_without_storage(gl, &desc, cty))
            };
            if let Some(data) = data_opt {
                try!(tex::init_texture_data(gl, name, desc, cty, data));
            }
            NewTexture::Texture(name)
        }else {
            let name = try!(tex::make_surface(gl, &desc, cty));
            NewTexture::Surface(name)
        };
        Ok(self.share.handles.borrow_mut().make_texture(object, desc))
    }

    fn view_buffer_as_shader_resource_raw(&mut self, hbuf: &handle::RawBuffer<R>)
                                      -> Result<handle::RawShaderResourceView<R>, f::ResourceViewError> {
        let gl = &self.share.context;
        let mut name = 0 as gl::types::GLuint;
        let buf_name = *self.frame_handles.ref_buffer(hbuf);
        let format = gl::R8; //TODO: get from the buffer handle
        unsafe {
            gl.GenTextures(1, &mut name);
            gl.BindTexture(gl::TEXTURE_BUFFER, name);
            gl.TexBuffer(gl::TEXTURE_BUFFER, format, buf_name);
        }
        let view = ResourceView::new_buffer(name);
        Ok(self.share.handles.borrow_mut().make_buffer_srv(view, hbuf))
    }

    fn view_buffer_as_unordered_access_raw(&mut self, _hbuf: &handle::RawBuffer<R>)
                                       -> Result<handle::RawUnorderedAccessView<R>, f::ResourceViewError> {
        Err(f::ResourceViewError::Unsupported) //TODO
    }

    fn view_texture_as_shader_resource_raw(&mut self, htex: &handle::RawTexture<R>, _desc: t::ResourceDesc)
                                       -> Result<handle::RawShaderResourceView<R>, f::ResourceViewError> {
        match self.frame_handles.ref_texture(htex) {
            &NewTexture::Surface(_) => Err(f::ResourceViewError::NoBindFlag),
            &NewTexture::Texture(t) => {
                //TODO: use the view descriptor
                let view = ResourceView::new_texture(t, htex.get_info().kind);
                Ok(self.share.handles.borrow_mut().make_texture_srv(view, htex))
            },
        }
    }

    fn view_texture_as_unordered_access_raw(&mut self, _htex: &handle::RawTexture<R>)
                                        -> Result<handle::RawUnorderedAccessView<R>, f::ResourceViewError> {
        Err(f::ResourceViewError::Unsupported) //TODO
    }

    fn view_texture_as_render_target_raw(&mut self, htex: &handle::RawTexture<R>, desc: t::RenderDesc)
                                         -> Result<handle::RawRenderTargetView<R>, f::TargetViewError> {
        self.view_texture_as_target(htex, desc.level, desc.layer)
            .map(|view| {
                let dim = htex.get_info().kind.get_level_dimensions(desc.level);
                self.share.handles.borrow_mut().make_rtv(view, htex, dim)
            })
    }

    fn view_texture_as_depth_stencil_raw(&mut self, htex: &handle::RawTexture<R>, desc: t::DepthStencilDesc)
                                         -> Result<handle::RawDepthStencilView<R>, f::TargetViewError> {
        self.view_texture_as_target(htex, desc.level, desc.layer)
            .map(|view| {
                let dim = htex.get_info().kind.get_level_dimensions(0);
                self.share.handles.borrow_mut().make_dsv(view, htex, dim)
            })
    }

    fn create_sampler(&mut self, info: t::SamplerInfo) -> handle::Sampler<R> {
        let name = if self.share.private_caps.sampler_objects_supported {
            tex::make_sampler(&self.share.context, &info)
        } else {
            0
        };
        let sam = FatSampler {
            object: name,
            info: info.clone(),
        };
        self.share.handles.borrow_mut().make_sampler(sam, info)
    }

    fn map_buffer_raw(&mut self, buf: &handle::RawBuffer<R>,
                      access: f::MapAccess) -> RawMapping {
        let gl = &self.share.context;
        let raw_handle = *self.frame_handles.ref_buffer(buf);
        unsafe { gl.BindBuffer(gl::ARRAY_BUFFER, raw_handle) };
        let ptr = unsafe { gl.MapBuffer(gl::ARRAY_BUFFER, match access {
            f::MapAccess::Readable => gl::READ_ONLY,
            f::MapAccess::Writable => gl::WRITE_ONLY,
            f::MapAccess::RW => gl::READ_WRITE
        }) } as *mut ::std::os::raw::c_void;
        RawMapping {
            pointer: ptr,
            target: gl::ARRAY_BUFFER
        }
    }

    fn unmap_buffer_raw(&mut self, map: RawMapping) {
        let gl = &self.share.context;
        unsafe { gl.UnmapBuffer(map.target) };
    }

    fn map_buffer_readable<T: Copy>(&mut self, buf: &handle::Buffer<R, T>)
                           -> d::mapping::Readable<T, R, Factory> {
        let map = self.map_buffer_raw(buf.raw(), f::MapAccess::Readable);
        self.map_readable(map, buf.len())
    }

    fn map_buffer_writable<T: Copy>(&mut self, buf: &handle::Buffer<R, T>)
                                    -> d::mapping::Writable<T, R, Factory> {
        let map = self.map_buffer_raw(buf.raw(), f::MapAccess::Writable);
        self.map_writable(map, buf.len())
    }

    fn map_buffer_rw<T: Copy>(&mut self, buf: &handle::Buffer<R, T>)
                              -> d::mapping::RW<T, R, Factory> {
        let map = self.map_buffer_raw(buf.raw(), f::MapAccess::RW);
        self.map_read_write(map, buf.len())
    }
}
