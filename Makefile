BIN_NAME = vxtr
BUILD_MODE = debug
BUILD_DIR = build

GLSLC_FLAGS = -O

SHADERS = $(wildcard shaders/*.*)
TARGET_DIR = $(realpath target/$(BUILD_MODE))
BIN = $(TARGET_DIR)/$(BIN_NAME)
DEP = $(BIN).d
BUILT_SHADERS = $(SHADERS:shaders/%=$(BUILD_DIR)/%.spv)
PWD = $(shell pwd)

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

all: $(BIN)

$(BIN): $(BUILT_SHADERS)
	cargo build $(CARGO_FLAGS)

$(BUILT_SHADERS): | $(BUILD_DIR)

# Alternative path is for Rust's depfiles
$(BUILD_DIR)/%.spv $(PWD)/src/../$(BUILD_DIR)/%: shaders/%
	glslc $(GLSLC_FLAGS) $^ -o $@

$(BUILD_DIR):
	mkdir -p $@

clippy:
	cargo clippy $(CARGO_FLAGS) -- -W clippy::all

clippy_pedantic:
	cargo clippy $(CARGO_FLAGS) -- -W clippy::pedantic

fmt:
	cargo fmt

clean:
	cargo clean
	rm -rf $(BUILD_DIR)

-include $(DEP)
.PHONY: run valgrind shaders all clippy_all clippy_pedantic fmt clean
