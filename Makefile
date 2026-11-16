# scop — one-line setup: `make` checks/installs dependencies, then builds (§ 16).
#
#   make              deps + release build
#   make run          run with OBJ (default: the 42 logo) and optional TEX (a .tga picture)
#                     e.g. make run OBJ=resources/teapot.obj TEX=resources/other.tga
#   make deps         check for (and where possible install) Rust, glslc, Vulkan
#   make sdk          download the LunarG Vulkan SDK (glslc) into your home folder — no admin
#   make spv          refresh the committed SPIR-V fallback in scop/shaders/spv/
#   make test         unit tests
#   make clean / fclean / re

OBJ ?= resources/42.obj
TEX ?=
SDK_DIR := $(HOME)/.local/vulkan-sdk

# Tools installed into the home folder (rustup, the SDK) must be found even in the
# same `make` run that installed them.
SDK_BIN := $(firstword $(wildcard $(SDK_DIR)/*/x86_64/bin))
export PATH := $(HOME)/.cargo/bin:$(if $(SDK_BIN),$(SDK_BIN):)$(PATH)

SHADERS := $(wildcard scop/shaders/*.vert scop/shaders/*.frag)

all: deps build

deps:
	@sh scripts/deps.sh

build:
	cargo build --release

run: build
	./target/release/scop $(OBJ) $(TEX)

test:
	cargo test

sdk:
	@sh scripts/deps.sh --install-sdk

spv:
	@command -v glslc >/dev/null || { echo "glslc not found: run 'make deps' (or 'make sdk')"; exit 1; }
	@mkdir -p scop/shaders/spv
	@for s in $(SHADERS); do \
		glslc "$$s" -o "scop/shaders/spv/$$(basename $$s).spv" && echo "compiled $$s"; \
	done

clean:
	cargo clean -p scop -p engine

fclean:
	cargo clean

re: fclean all

.PHONY: all deps build run test sdk spv clean fclean re
