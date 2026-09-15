Linux builds and body-frame target allocation.

- Linux x64 native executable and tar.gz, built on Ubuntu 22.04 (glibc 2.35+). X11/Wayland desktop and OpenGL required. Run ./silentswim-studio after extracting the tarball.
- Windows x64 executable and ZIP remain available.
- Allocation starts with editable Fx/Fy/Fz [N] and Mx/My/Mz [N m]. The minimum-norm baseline, fin forces, null-space redistribution, and realized body wrench can be inspected in order.
- Target and realized values appear together. Targets are not silently scaled; invalid baselines and final commands are shown explicitly. Reset distribution preserves the body target.
- Short technical headings replace slogans and the large header subtitle.
- Scene v2 stores body targets; v1 preserves legacy references. CSV export adds all six body components, and Linux video exports include a shell encoder script.

Both platforms run numerical tests and Clippy. Linux additionally renders ten native views under Xvfb/Mesa before publication, including zero, signed, high valid and invalid body targets.

Interactive numbers are illustrative. The minimum-norm baseline does not solve actuator-constrained allocation; a baseline failure does not certify target infeasibility. Executables do not control hardware. See README for platform dependencies. SHA256SUMS.txt covers all binaries and archives.
