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
        steps = [step for step, _ in arrays[0]]  # Extract steps from the first iteration
        sums = {step: 0 for step in steps}  # Initialize sums dictionary with step names

        # Sum up the durations for each step across all subarrays
        for subarray in arrays:
            for step_name, duration in subarray:
                sums[step_name] += duration

        # Convert the sums dictionary to two lists (for plotting)
        step_names = list(sums.keys())
        durations = list(sums.values())
        return step_names, durations

    # Calculate sums for both arrays of subarrays
    step_names1, sums1 = calculate_sums(arrays1)
    step_names2, sums2 = calculate_sums(arrays2)

    # Create two subplots, one on top of the other
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 8))

    # First plot (top) for the first array of subarrays
    ax1.plot(step_names1, sums1, marker='o', linestyle='-', color='b')
    ax1.set_title('Total Duration per Step - Main Loop')
    ax1.set_xlabel('Step Name')
    ax1.set_ylabel('Total Duration')
    ax1.grid(True)

    # Second plot (bottom) for the second array of subarrays
    ax2.plot(step_names2, sums2, marker='o', linestyle='-', color='r')
    ax2.set_title('Total Duration per Step - Result Processor')
    ax2.set_xlabel('Step Name')
    ax2.set_ylabel('Total Duration')
    ax2.grid(True)

    # Adjust layout so the plots don't overlap
    plt.tight_layout()

    # Display the plot
    plt.show()

# Load arrays from the JSON files
arrays1 = load_json_data('/tmp/windmill/profiling_main.json')
arrays2 = load_json_data('/tmp/windmill/profiling_result_processor.json')

# Plot the data
plot_two_arrays_of_subarrays(arrays1, arrays2)