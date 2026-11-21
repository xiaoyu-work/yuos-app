#define _POSIX_C_SOURCE 200809L

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char **argv)
{
    if (argc != 2 || strcmp(argv[1], "--gui") != 0) {
        fputs("Yu Office must be started with its managed GUI entry.\n", stderr);
        return 2;
    }
    char path[PATH_MAX];
    const ssize_t length = readlink("/proc/self/exe", path, sizeof(path) - 1);
    if (length < 1 || (size_t)length == sizeof(path) - 1) {
        fputs("Cannot locate the Yu Office package.\n", stderr);
        return 1;
    }
    path[length] = '\0';
    char *base = strrchr(path, '/');
    const char runtime[] = "/../resources/usr/share/yu-office/program/soffice.bin";
    if (base == NULL || (size_t)(base - path) + sizeof(runtime) > sizeof(path)) {
        fputs("Cannot locate the Yu Office runtime.\n", stderr);
        return 1;
    }
    memcpy(base, runtime, sizeof(runtime));
    if (setenv("SAL_ENABLE_FILE_LOCKING", "1", 1) != 0 ||
        setenv("SAL_USE_VCLPLUGIN", "gtk3", 1) != 0) {
        perror("Yu Office environment");
        return 1;
    }
    execl(path, path, "--nologo", (char *)NULL);
    perror("Yu Office runtime");
    return 1;
}
