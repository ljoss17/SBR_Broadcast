from os import listdir
from os.path import isfile, join
import os
import argparse

parser = argparse.ArgumentParser(description="Check if all N processors delivered the message.")
parser.add_argument("-n", type=int, help="Size of the system, N.")
parser.add_argument("-i", type=str, help="ID of the test to parse.")

args = parser.parse_args()

N = args.n
id = args.i

folder_path = join(os.getcwd(), f"check_{id}")
onlyfiles = [f for f in listdir(folder_path) if isfile(join(folder_path, f))]

missing = 0
m_files = []

for i in range(N):
    f_name = f"tmp_{i}.txt"
    if not(f_name in onlyfiles):
        missing += 1
        m_files.append(i)

log_file = join(os.getcwd(), f"output_{id}.log")

f = open(log_file, "r")
lines = f.readlines()

setup_start = 0
setup_end = 0
send_start = 0

for i in range(len(lines)):
    if " - " in lines[i]:
        info = lines[i].split(" - ")
        if info[1] == "Start\n":
            setup_start = int(info[0])
        if info[1] == "Trigger send\n":
            setup_end = int(lines[i-1].split(" - ")[0])
            send_start = int(info[0])

send_end = int(lines[-1].split(" - ")[0])

result = ""

if missing > 0:
    result = f"ERROR, missing {missing} files\nFiles : {m_files}\n"
else:
    result = "SUCCESS\n"

with open(f"results_{id}.log", "w") as result_file:
    result_file.write(result)
    result_file.write(f"Setup time : {setup_end-setup_start} seconds\n")
    result_file.write(f"Send time : {send_end-send_start} seconds")
