N=1024
G=30
E=610
E_THR=470
echo "Results:" > results.txt
for i in $(seq 1 30)
do
    echo "Output:" > output_$i.log
    echo $i
    rm check/*.txt
    cargo run $N $G $E $E_THR >> output_$i.log 2>&1 &
    RUST_ID=$!
    sleep 60
    kill -9 $RUST_ID
    python test_murmur.py -n=$N >> results.txt
done
