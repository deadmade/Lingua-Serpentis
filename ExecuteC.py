from cffi import FFI

def execute_c(result):
    ffi = FFI()

    c_code = """
    #include <stdio.h>
    
    void hello() {
        printf("Hello from C!\\n");
    }
"""

    ffi.cdef("void hello();")  # Declare the function signature

    lib = ffi.dlopen(None)  # Load the compiled module
    ffi.set_source("_my_c_module", c_code)  # Compile in-memory

    ffi.compile(verbose=True)  # Compile and link dynamically


