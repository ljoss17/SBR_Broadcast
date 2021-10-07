N=1024
G=30
E=610
E_THR=470
R=230
R_THR=40
D=230
D_THR=80
echo "Results:" > results.txt
for i in $(seq 1 5)
do
    echo "Output:" > output_$i.log
    echo $i
    rm check/*.txt
    cargo run $N $G $E $E_THR $R $R_THR $D $D_THR >> output_$i.log 2>&1 &
    RUST_ID=$!
    sleep 120
    kill -9 $RUST_ID
    python test_murmur.py -n=$N >> results.txt
done
