(cd .. && go build -buildmode=c-archive -o libmpt.a witness_gen_wrapper.go && cp libmpt.a libmpt.h rust_call/build)
cargo run 
