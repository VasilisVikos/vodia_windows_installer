
## Changelog: Vodia PBX Installer Wizard v1.3

## Version 1.3

### Added
    
    - Added an Uninstall option to remove an existing Vodia PBX installation.
    - Added uninstall support for:
        - stopping the PBX Windows service
        - deleting the PBX Windows service
        - removing the Vodia PBX install folder
        - removing Vodia-related Windows Firewall rules
    - Added duplicate-install protection.
        - The installer now checks whether the PBX service already exists.
        - The installer now checks whether an existing PBX installation is already present in the selected install folder.
        - If either is found, installation is stopped and the user is instructed to uninstall first.
    - Added cleanup of the temporary downloaded/staged installation folder after a successful install.
    - Added clearer first-run messaging while the PBX starts once to generate its initial configuration.
    - Added CHANGELOG

### Changed

    - Improved install flow so staged download cleanup happens only after the elevated installer has completed its file copy and setup process.
    - Improved error visibility in the elevated installer window.
        - Successful installs close normally.
        - Failed installs remain visible so the user can read the error.
    - Updated password generation to better match the Linux installer template style.
        - The installer generates a random alphanumeric admin password.
        - The password is written to setup.json as an MD5 hash.
        - The plaintext password is saved in installation.txt for the user.
    - Updated setup.json generation to use local PBX settings only.
        - Removed cloud-specific redirect fields.
        - Prevents accidental redirect to hosted/cloud PBX addresses.

### Fixed

    - Fixed an issue where the staged download folder could be deleted too early, causing the elevated installer to fail after download.
    - Fixed duplicate installation behavior where the installer could attempt to reuse or overwrite an existing PBX service.
    - Fixed uninstall flow so the window exits properly after the user confirms completion.
    - Fixed installer behavior so existing installations are handled safely instead of silently continuing.
    - Improved handling around first-run PBX initialization and pbx.xml creation.

### Notes

    - This release is Windows-only.
    - Users should uninstall an existing PBX installation before installing again with v1.1.
    - Installation details and generated admin credentials are saved to:
       ```C:\Program Files\Vodia\PBX\installation.txt```

## Version 1.2

    - Automated installation process with Vodia's setup.json.
    - Added README.

## Version 1.1

    - Created front end for CLI. 

## Version 1.0

    - Released a Windows based installer for Vodia PBX. 