BIN_NAME = vulkan_tutorial
BUILD_MODE = debug

SHADERS = $(wildcard shaders/*.vert) $(wildcard shaders/*.frag)
TARGET_DIR = $(realpath target/$(BUILD_MODE))
BIN = $(TARGET_DIR)/$(BIN_NAME)
DEP = $(BIN).d

BUILT_SHADERS = $(SHADERS:%=%.spv)

ifeq "$(BUILD_MODE)" "release"
    CARGO_FLAGS = --release
else ifneq "$(BUILD_MODE)" "debug"
    $(error Unknown build mode "$(BUILD_MODE)", acceptable modes are "debug" and "release")
endif

run: $(BIN)
	$(BIN)

valgrind: $(BIN)
	valgrind --leak-check=full $(BIN)

shaders: $(BUILT_SHADERS)

build: $(BIN)

$(BIN): $(BUILT_SHADERS)
	cargo build $(CARGO_FLAGS)

%.spv: %
	glslc $^ -o $@

clippy:
	cargo clippy $(CARGO_FLAGS) -- -W clippy::all

clippy_pedantic:
	cargo clippy $(CARGO_FLAGS) -- -W clippy::pedantic

fmt:
	cargo fmt

clean:
	cargo clean
	rm -f $(BUILT_SHADERS)

-include $(DEP)
.PHONY: run valgrind shaders build clippy_all clippy_pedantic fmt clean
