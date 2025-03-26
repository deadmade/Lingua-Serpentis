import subprocess


def execute_c(c_code):
    # 1. Schreibe den C-Code in eine Datei
    with open("output.c", "w") as f:
        f.write(c_code)

    # 2. Kompiliere die Datei mit gcc
    compile_result = subprocess.run(["gcc", "output.c", "-o", "out"], capture_output=True, text=True)

    if compile_result.returncode != 0:
        print("❌ Fehler beim Kompilieren:")
        print(compile_result.stderr)
        return

    print("✅ Kompilierung erfolgreich. Starte das Programm:\n")

    # 3. Führe das kompilierte Programm aus
    run_result = subprocess.run(["./out"], capture_output=True, text=True)

    if run_result.returncode != 0:
        print("❌ Fehler beim Ausführen:")
        print(run_result.stderr)
    else:
        print(run_result.stdout)
