import os

from os.path import join
import argparse

parser = argparse.ArgumentParser(description="Parse a log file and translate Identites to human readable values.")
parser.add_argument("-f", type=str, help="Name of the file to be parsed.")
parser.add_argument("-n", type=int, help="Size of the system, N.")

args = parser.parse_args()

file_name = join(os.getcwd(), args.f)

f = open(file_name, "r")

lines = f.readlines()

identities = {}

for i in range(args.n):
    line = lines[i].split("<", 1)[1].split(">")
    id = line[1].split(" : KC : Identity(")[1].replace(")\n", "")
    identities[id] = line[0]

with open(file_name, "r") as old_file:
    filedata = old_file.read()

for i in identities:
    filedata = filedata.replace(i, identities.get(i))

new_filename = file_name+".parsed"

with open(new_filename, "w") as new_file:
    new_file.write(filedata)
