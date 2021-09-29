# Otustenverkko
IoT framework from embedded environmental telemetry device to the cloud and smartphone application.

## Project mission
The project is aimed to provide minimal IoT framework based on Google Cloud Platform with embedded client, cloud functions and smartphone app.

These are the targets for the project:
- Easy to replicate environment for hackers who want to use the framework in their projects
  - This should work as boilerplate example for anybody wanting to tailor the solution for them.
- Stay inside GCP free tier limits to make this zero cost solution for the most for private use
- Use Rust and Flutter as the main programming language
  - Python as the stepping stone in cloud services but it should be left for doing DevOps in the future

## Components
### Embedded node
Embedded node is featching data from the environmental sensors to push them to the cloud using MQTT.

### Cloud functions
GCP is used for storing the data into the cloud and pushing the snapshot into Firebase.

Firebase is used for storing the embedded device model that's made visible through the smartphone application.

### Flutter smartphone application
Smartphone application is the window into the device and telemetry. The application is implemented using Flutter to support both Android and iOS. The longer term target is to make the asset to work as web front-end application and even support PC computers.

### Cloud deployment tools
These tools are used for setting up provision the device and the cloud environment without the user needing to know all the details.

## Status
This project is in very early phase and doesn't provide E2E functionality today.

## Building the system
### Device hardware
TBD

### Setting up the cloud
TBD

### Smartphone application
TBD
