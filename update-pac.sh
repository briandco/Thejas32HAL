#!/usr/bin/env bash
set -exuo pipefail
REPO_ROOT=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Regenerate SVD from peripheral definitions
cd ${REPO_ROOT}/tools/thejas32-svd
uv run main.py -o ${REPO_ROOT}/thejas32-pac/thejas32.svd

# Generate PAC source from SVD
tmp_dir=$(mktemp -d -t svd2rust-XXXX)
pushd ${tmp_dir}
svd2rust -i ${REPO_ROOT}/thejas32-pac/thejas32.svd -c ${REPO_ROOT}/thejas32-pac/svd2rust.toml --target riscv
form -i lib.rs -o src
rustfmt src/lib.rs
rm -rf ${REPO_ROOT}/thejas32-pac/src
mv src ${REPO_ROOT}/thejas32-pac/src
mv build.rs ${REPO_ROOT}/thejas32-pac/build.rs
mv device.x ${REPO_ROOT}/thejas32-pac/device.x
popd
rm -rf ${tmp_dir}

cd ${REPO_ROOT}/thejas32-pac
cargo fmt
