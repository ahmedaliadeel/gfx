initSidebarItems({"constant":[["DEPTH_STENCIL","The resource can serve as a depth/stencil target."],["RENDER_TARGET","The resource can be rendered into."],["SHADER_RESOURCE","The resource can be bound to the shader for reading."],["UNORDERED_ACCESS","The resource can be bound to the shader for writing."]],"enum":[["BufferError","Error creating a buffer."],["BufferRole","Role of the memory buffer. GLES doesn't allow chaning bind points for buffers."],["BufferUpdateError","An error happening on buffer updates."],["CombinedError","An error from creating textures with views at the same time."],["IndexBuffer","Type of index-buffer used in a Slice."],["LayerError","An error associated with selected texture layer."],["MapAccess","Specifies the access allowed to a buffer mapping."],["PipelineStateError","Error creating a PipelineState"],["Primitive","Describes what geometric primitives are created from vertex data."],["ResourceViewError","Error creating either a ShaderResourceView, or UnorderedAccessView."],["ShaderSet","A complete set of shaders to link a program."],["TargetViewError","Error creating either a RenderTargetView, or DepthStencilView."],["UniformValue","A value that can be uploaded to the device as a uniform."],["UpdateError","An error occuring in buffer/texture updates."],["Usage","A hint as to how this buffer/texture will be used."]],"fn":[["cast_slice","Cast a slice from one POD type to another."]],"macro":[["gfx_constant_struct",""],["gfx_defines","Defines vertex, constant and pipeline formats in one block"],["gfx_format",""],["gfx_impl_struct",""],["gfx_pipeline",""],["gfx_pipeline_base",""],["gfx_pipeline_inner",""],["gfx_vertex_struct",""]],"mod":[["format","Universal format specification. Applicable to textures, views, and vertex buffers."],["handle","Device resource handles"],["macros","Convenience macros Various helper macros."],["preset","State presets"],["pso","A typed high-level graphics pipeline interface."],["shade","Shaders Shader parameter handling."],["state","Fixed-function hardware state."],["tex","Texture creation and modification."],["traits","public re-exported traits"]],"struct":[["Bind","Bind flags"],["BufferInfo","An information block that is immutable and associated with each buffer."],["DomainShader",""],["Encoder","Graphics Command Encoder"],["GeometryShader",""],["HullShader",""],["PixelShader",""],["ProgramInfo","Metadata about a program."],["Slice","A `Slice` dictates in which and in what order vertices get processed. It is required for processing a PSO."],["VertexShader",""]],"trait":[["CommandBuffer","An interface of the abstract command buffer. It collects commands in an efficient API-specific manner, to be ready for execution on the device."],["Device","A `Device` is responsible for submitting `CommandBuffer`s to the GPU."],["Factory","A `Factory` is responsible for creating and managing resources for the context it was created with. "],["IntoIndexBuffer","A helper trait to create `IndexBuffers` from different kinds of data."],["Resources","Different types of a specific API."],["Typed","A service trait used to get the raw data out of strong types. Not meant for public use."]],"type":[["InstanceCount",""],["InstanceOption",""],["VertexCount",""]]});