from os import listdir
from os.path import isfile, join
import os
import argparse

parser = argparse.ArgumentParser(description="Check if all N processors delivered the message.")
parser.add_argument("-n", type=int, help="Size of the system, N.")

args = parser.parse_args()

N = args.n

folder_path = join(os.getcwd(), "check")
onlyfiles = [f for f in listdir(folder_path) if isfile(join(folder_path, f))]

missing = 0
m_files = []

for i in range(N):
    f_name = f"{i}.txt"
    if not(f_name in onlyfiles):
        missing += 1
        m_files.append(i)

if missing > 0:
    print(f"ERROR, missing {missing} files")
    print(f"Files : {m_files}")
else:
    print("SUCCESS")
