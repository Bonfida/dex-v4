import os
import matplotlib.pyplot as plt

log_file = open("out.log", "r")
serum_dex_program_id = ""
aob_dex_program_id = ""

serum_nb_instr = []
aob_nb_instr = []
collect_data = False
for line in log_file:
    if "Serum_dex_key" in line:
        serum_dex_program_id = line.split(" ")[1].strip("\n")
        continue
    if "Aob_dex_key" in line:
        aob_dex_program_id = line.split(" ")[1].strip("\n")
        continue
    if "New Order" in line:
        collect_data = True
    if collect_data:
        if (serum_dex_program_id + " consumed") in line:
            serum_nb_instr.append(int(line.split(" ")[6]))
        if (aob_dex_program_id + " consumed") in line:
            aob_nb_instr.append(int(line.split(" ")[6]))
print(serum_nb_instr, aob_nb_instr)
plt.plot(serum_nb_instr, label=("Serum dex"))
plt.plot(aob_nb_instr, label=("Aob dex"))
plt.show()
