
init:
  git submodule update --init
  cargo build --package=obs-sys


@obs-test:
  env OBS_PLUGINS_PATH=$(pwd)/target/release \
    OBS_PLUGINS_DATA_PATH=$(pwd)/target/release \
    obs

