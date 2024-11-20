import json
import matplotlib.pyplot as plt

# Function to load JSON data from a file
def load_json_data(filepath):
    with open(filepath, 'r') as file:
        data = json.load(file)
    return data

# Function to plot two arrays of subarrays with tuples (step_name, duration)
def plot_two_arrays_of_subarrays(arrays1, arrays2):
    # Function to calculate sum of durations for each step
    def calculate_sums(arrays):
        steps = [step for step, _ in arrays[0]['timings']]  # Extract steps from the first iteration
        sums = {step: 0.0 for step in steps}  # Initialize sums dictionary with step names

        # Sum up the durations for each step across all subarrays
        for subarray in arrays:
            for step_name, duration in subarray['timings']:
                if step_name not in sums:
                    sums[step_name] = 0
                sums[step_name] += duration

        for step_name, duration in sums.items():
            sums[step_name] = duration / 1000000000

        # Convert the sums dictionary to two lists (for plotting)
        step_names = list(sums.keys())
        durations = list(sums.values())
        return step_names, durations

    # Calculate sums for both arrays of subarrays
    step_names1, sums1 = calculate_sums(arrays1)
    step_names2, sums2 = calculate_sums(arrays2)

    # Create two subplots, one on top of the other
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 12))

    # First plot (top) for the first array of subarrays
    ax1.bar(step_names1, sums1, color='b')
    ax1.set_title('Total Duration per Step - Main Loop')
    ax1.set_xlabel('Step Name')
    ax1.set_ylabel('Total Duration (s)')
    ax1.grid(True, axis='y')
    ax1.tick_params(axis='x', rotation=45)

    # Second plot (bottom) for the second array of subarrays
    ax2.bar(step_names2, sums2, color='r')
    ax2.set_title('Total Duration per Step - Result Processor')
    ax2.set_xlabel('Step Name')
    ax2.set_ylabel('Total Duration (s)')
    ax2.grid(True, axis='y')
    ax2.tick_params(axis='x', rotation=45)

    # Adjust layout so the plots don't overlap
    plt.tight_layout()

    # Display the plot
    plt.show()

# Load arrays from the JSON files
main = load_json_data('/tmp/windmill/profiling_main.json')
result_processor = load_json_data('/tmp/windmill/profiling_result_processor.json')

arrays1 = main['timings']
arrays2 = result_processor['timings']

total_duration1 = main['total_duration']/1000
total_duration2 = result_processor['total_duration']/1000

print(f"Total duration for main: {total_duration1}s")
print(f"Total duration for result processor: {total_duration2}s")

iterations_total = sum(main['iter_durations']) / 1000000000
iterations_total2 = sum(result_processor['iter_durations']) / 1000000000

print(f"Number of iterations: {len(main['iter_durations'])}")
print(f"Total iterations for main: {iterations_total}s")
print(f"Total iterations for result processor: {iterations_total2}s")

# Calculate RPS
rps1 = len(main['iter_durations']) / total_duration1
rps2 = len(result_processor['iter_durations']) / total_duration2

print(f"RPS for main: {rps1}")
print(f"RPS for result processor: {rps2}")

# Plot the data
plot_two_arrays_of_subarrays(arrays1, arrays2)