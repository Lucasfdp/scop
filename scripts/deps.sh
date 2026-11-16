#!/bin/sh
# scop dependency check (§ 16): Rust, glslc, Vulkan.
# Installs what it safely can (no admin rights needed); otherwise says exactly
# what's missing and how to get it. Exit code 1 only if the build can't work.
#
#   sh scripts/deps.sh                 check (and install where possible)
#   sh scripts/deps.sh --install-sdk   also download the LunarG Vulkan SDK (glslc)

SDK_DIR="$HOME/.local/vulkan-sdk"
SDK_URL="https://sdk.lunarg.com/sdk/download/latest/linux/vulkan_sdk.tar.xz"
OS=$(uname -s)
failed=0

ok()   { printf '  \033[32mok\033[0m       %s\n' "$1"; }
warn() { printf '  \033[33mwarning\033[0m  %s\n' "$1"; }
bad()  { printf '  \033[31mmissing\033[0m  %s\n' "$1"; failed=1; }
has()  { command -v "$1" >/dev/null 2>&1; }

add_sdk_to_path() {
    for bin in "$SDK_DIR"/*/x86_64/bin; do
        [ -d "$bin" ] && PATH="$bin:$PATH"
    done
}

PATH="$HOME/.cargo/bin:$PATH"
add_sdk_to_path

install_sdk() {
    echo "  downloading the LunarG Vulkan SDK (~250 MB) into $SDK_DIR ..."
    mkdir -p "$SDK_DIR" &&
        curl -fL "$SDK_URL" -o "$SDK_DIR/sdk.tar.xz" &&
        tar -xJf "$SDK_DIR/sdk.tar.xz" -C "$SDK_DIR" &&
        rm -f "$SDK_DIR/sdk.tar.xz" &&
        add_sdk_to_path
}

echo "scop: checking dependencies ($OS)"

# 1. Rust (cargo): always installable into the home folder, no admin needed.
if has cargo; then
    ok "Rust ($(cargo --version))"
elif has curl; then
    echo "  installing Rust with rustup into ~/.cargo ..."
    if curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal; then
        ok "Rust installed ($("$HOME/.cargo/bin/cargo" --version))"
    else
        bad "Rust: install failed, see https://rustup.rs"
    fi
else
    bad "Rust: install it from https://rustup.rs"
fi

# 2. glslc (GLSL -> SPIR-V). Optional: without it the build uses the committed
#    SPIR-V in scop/shaders/spv/ (option C).
if [ "$1" = "--install-sdk" ] && [ "$OS" = Linux ] && ! has glslc; then
    install_sdk || warn "SDK download failed"
fi
if has glslc; then
    ok "glslc ($(command -v glslc))"
elif [ "$OS" = Darwin ] && has brew; then
    echo "  installing glslc with Homebrew ..."
    if brew install shaderc; then ok "glslc installed"; else warn "glslc: brew install shaderc failed"; fi
elif ls scop/shaders/spv/*.spv >/dev/null 2>&1; then
    warn "glslc not found: the build will use the committed SPIR-V."
    warn "  To get the compiler without admin rights: make sdk"
else
    bad "glslc not found and no committed SPIR-V: run make sdk"
fi

# 3. Vulkan (the loader; MoltenVK on macOS).
if [ "$OS" = Darwin ]; then
    found=""
    for dir in "${VULKAN_SDK:-/nonexistent}/lib" "$(brew --prefix 2>/dev/null)/lib" /usr/local/lib; do
        if [ -e "$dir/libvulkan.1.dylib" ]; then found="$dir"; break; fi
    done
    if [ -n "$found" ]; then
        ok "Vulkan loader ($found/libvulkan.1.dylib)"
    elif has brew; then
        echo "  installing Vulkan (MoltenVK + loader) with Homebrew ..."
        if brew install molten-vk vulkan-loader; then
            ok "Vulkan installed"
        else
            bad "Vulkan: brew install molten-vk vulkan-loader failed"
        fi
    else
        bad "Vulkan: install Homebrew, then: brew install molten-vk vulkan-loader"
    fi
else
    # ldconfig often lives in /sbin, outside a normal user's PATH; also look directly.
    found=""
    if { ldconfig -p 2>/dev/null || /sbin/ldconfig -p 2>/dev/null; } | grep -q 'libvulkan\.so\.1'; then
        found="system"
    fi
    for lib in /usr/lib/x86_64-linux-gnu /usr/lib64 /usr/lib "${VULKAN_SDK:-/nonexistent}/lib"; do
        if [ -z "$found" ] && [ -e "$lib/libvulkan.so.1" ]; then found="$lib"; fi
    done
    if [ -n "$found" ]; then
        ok "Vulkan loader (libvulkan.so.1, $found)"
    else
        bad "Vulkan loader not found: ask staff for package libvulkan1, or try: make sdk"
    fi
fi

if [ "$failed" -eq 0 ]; then
    echo "scop: dependencies OK"
else
    echo "scop: some dependencies are missing (see above)"
fi
exit "$failed"
