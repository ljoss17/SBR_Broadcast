import subprocess
import time

for i in range(100):
    print(f"{i}:")
    rendezvous = subprocess.Popen(["cargo", "run", "--bin", "rendezvous"], stdin=subprocess.PIPE)
    time.sleep(10)
    f = open(f"output_{i}.log", "w")
    sbr = subprocess.Popen(["cargo", "run"], stdin=subprocess.PIPE, stdout=f)
    time.sleep(100)
    print("Send trigger")
    try:
        sbr.communicate(input="send\n".encode(), timeout=10)
    except subprocess.TimeoutExpired:
        print("timeout")
    time.sleep(120)
    print("Will stop")
    sbr.kill()
    rendezvous.kill()
    subprocess.run(["sh", "run_clean.sh", f"{i}"])
    subprocess.run(["python", "parse_output.py", "-n", "100", "-i", f"{i}"])
