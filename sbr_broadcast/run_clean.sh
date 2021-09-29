rm check/*.txt
cargo run &
RUST_ID=$!
sleep 5
kill -9 $RUST_ID
python test_murmur.py
