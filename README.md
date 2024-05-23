# Energyleaf PI

This repository serves as a binding layer between the Energyleaf backend and a Hichi sensor.

## Installation

To set up Energyleaf PI, follow these steps:

### 1. Download the Binary

Begin by downloading the binary file provided in this repository. The binary contains the executable code necessary to run the program.

### 2. Create a .env File

In the same directory where you've downloaded the binary, create a new file named `.env`. This file will store environment variables needed for the proper functioning of the program.

### 3. Set Environment Variable Values

Inside the `.env` file, set the following key-value pairs:

```bash
# The URL from which sensor data is pulled
SENSOR_URL=

# The admin URL, for example: http://localhost:3001/api/v1
ADMIN_URL=
```

Replace SENSOR_URL with the URL from which sensor data will be retrieved, and ADMIN_URL with the admin URL where Energyleaf PI will interact with the backend.

### 4. Save the .env File

Once you've set the desired environment variable values in the .env file, save it. Ensure that it is saved in the same directory as the downloaded binary.

By following these steps, you'll have the necessary setup to effectively use Energyleaf PI.
