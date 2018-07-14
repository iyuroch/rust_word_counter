import subprocess
import json
import hashlib
import csv

CONFIG_FILE = "config.json"

def md5(fname):
    hash_md5 = hashlib.md5()
    with open(fname, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            hash_md5.update(chunk)
    return hash_md5.hexdigest()

def main():
    config_dict = {
        "read_file": "big_file.txt",
        "threads": 1,
        "count_jobs": 1,
        "merge_jobs": 1,
        "alph_count": "alph_count.txt",
        "nume_count": "nume_count.txt"
    }
    # the order in out csv file will be:
    # count_jobs, dict_jobs, threads, real, user, sys, md5hash


    csv_file = open('dict.csv', 'wb')
    writer = csv.writer(csv_file)
    writer.writerow(["count_jobs", "dict_jobs", "threads", "real", "user", "sys", "md5hash"])

    for count_jobs in range(1, 4):
        for dict_jobs in range(1, 4):
            for threads in range(1, 4):

                config_dict["count_jobs"], config_dict["dict_jobs"], config_dict["threads"] = count_jobs, dict_jobs, threads

                with open('config.json', 'w') as outfile:
                    json.dump(config_dict, outfile)

                print(config_dict)

                res_row = [count_jobs, dict_jobs, threads]

                cmd = "time -p ./word_counter"
                exec_time = {
                        "real": float("inf"),
                        "user": float("inf"),
                        "sys": float("inf")
                        }
                for i in range(5):
                    #we run this multiple time to find lowest time
                    p = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                    out = p.stdout.read()
                    for line in out.split("\n"):
                        if line != "" and line != "true":
                            try:
                                time_name, value = line.split(" ")
                                exec_time[time_name] = min(exec_time[time_name], float(value))
                            except Exception as e:
                                print(e, out)
                    print(exec_time)

                res_row.extend([exec_time["real"], 
                            exec_time["user"], exec_time["sys"]])
                res_row.append(md5(config_dict["alph_count"]))
                writer.writerow(res_row)



    csv_file.close()
    # we need to return to default
    config_dict = {
        "read_file": "big_file.txt",
        "threads": 1,
        "count_jobs": 1,
        "merge_jobs": 1,
        "alph_count": "alph_count.txt",
        "nume_count": "nume_count.txt"
    }   
    with open('data.json', 'w') as outfile:
        json.dump(config_dict, outfile)    #write initial config_dict to config_file



if __name__ == "__main__":
    main()
