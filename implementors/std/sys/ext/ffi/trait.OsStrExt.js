(function() {var implementors = {};
implementors["shared_library"] = [];implementors["gfx_gl"] = [];implementors["libloading"] = [];implementors["tempfile"] = [];implementors["winit"] = [];implementors["glutin"] = [];implementors["sdl2"] = [];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        
})()
