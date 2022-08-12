# Dinnery `tr-gen`

### Overview
This app alows you to generate json file with translations based on your Google Spreadheet!

#### Video

https://user-images.githubusercontent.com/53144702/184351550-c82dc02c-204d-4736-85a1-4daed7353bfc.mp4




### Building
#### Prepare your `.env` file:
- Get spreadsheet's ID - that's the file you want to run the script on
- Obtain client secret and client id on Google's API console page:
- https://console.cloud.google.com/apis/credentials
- Prepare OAuth consent screen:
- https://console.cloud.google.com/apis/credentials/consent
- Update the following lines to your `.env` file:
```
SPREADSHEET_ID=<your spreadsheet's ID>
CLIENT_ID=<your client id>
CLIENT_SECRET=<your client secret>
```
Remember to add `spreadsheets.readonly` scope to your OAuth consent screen.
Also, remember to add your Google Account with the spritesheet to test account in the OAuth consent screen section.

#### Build project
`cargo build --release`

### Usage
```shell
USAGE:
    tr-gen [OPTIONS] --app <APP>

OPTIONS:
    -a, --app <APP>          Name of the sheet to use (eg. `landing_page`)
    -h, --help               Print help information
    -o, --output <OUTPUT>    Path to the output file [default: translations.json]
    -V, --version            Print version information
```
