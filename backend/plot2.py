import json
import matplotlib.pyplot as plt

# Path to the profiling JSON file
# file_path = "/tmp/windmill/profiling_main.json"
file_path = "/tmp/profiling.json"

# Load the JSON data
with open(file_path, "r") as f:
    data = json.load(f)

# Extract timings for "pre pull->post pull"
pre_post_pull_timings = [
    timing / 1000000.0 for entry in data["timings"]
    for step, timing in entry["timings"]
    # if step == "pre pull->post pull"
    if step == "->job pulled from DB"
]

# Plotting the distribution
plt.figure(figsize=(10, 6))
# plt.hist(pre_post_pull_timings, bins=10, edgecolor='black')
plt.scatter(range(len(pre_post_pull_timings)), pre_post_pull_timings,
                alpha=1.0,          # Transparency level
                s=40)                # Size of the dots`)
plt.title("Distribution of 'pre pull->post pull' timings")
# plt.xlabel("Time (ms)")
# plt.ylabel("Frequency")
plt.xlabel("Sample Index")
plt.ylabel("Time (ms)")
plt.grid(True)
plt.tight_layout()
plt.show()