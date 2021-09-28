from os import listdir
from os.path import isfile, join
import os

folder_path = join(os.getcwd(), "check")
onlyfiles = [f for f in listdir(folder_path) if isfile(join(folder_path, f))]

for i in range(100):
    f_name = f"{i}.txt"
    if not(f_name in onlyfiles):
        print(f"ERROR for file {i}")
        exit()

print("SUCCESS")
