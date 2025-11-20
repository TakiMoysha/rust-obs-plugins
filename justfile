
init:
  git submodule update --init
  cargo build --all


[doc("ex: just test avatar-plugin")]
test package *ARGS:
  cargo test --package={{package}} {{ARGS}}

[doc("ex: just build avatar-plugin")]
build plugin *ARGS:
  cargo build --release --package={{plugin}} {{ARGS}}
    
@obs-test:
  env OBS_PLUGINS_PATH=$(pwd)/target/release \
    OBS_PLUGINS_DATA_PATH=$(pwd)/target/release \
    obs

