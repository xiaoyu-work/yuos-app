cmake_minimum_required(VERSION 3.25)
project(yu-office LANGUAGES C)

find_package(Python3 REQUIRED COMPONENTS Interpreter)
set(YU_OFFICE_JOBS "3" CACHE STRING "Parallel LibreOffice build jobs")
set(runtime "${CMAKE_CURRENT_BINARY_DIR}/office-runtime")
add_custom_target(office-runtime ALL
    COMMAND "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/build.py"
        --output "${runtime}" --jobs "${YU_OFFICE_JOBS}"
    USES_TERMINAL
    VERBATIM
)

add_executable(yu-office launcher.c)
target_compile_features(yu-office PRIVATE c_std_11)
add_dependencies(yu-office office-runtime)
install(CODE [[
    if(NOT IS_ABSOLUTE "$ENV{DESTDIR}" OR "$ENV{DESTDIR}" STREQUAL "/")
        message(FATAL_ERROR "Yu Office requires a private absolute DESTDIR")
    endif()
]])
install(TARGETS yu-office RUNTIME DESTINATION bin)
install(DIRECTORY "${runtime}/" DESTINATION share/yu-office
    PATTERN "soffice.bin" EXCLUDE PATTERN "xpdfimport" EXCLUDE)
install(PROGRAMS "${runtime}/program/soffice.bin" "${runtime}/program/xpdfimport"
    DESTINATION share/yu-office/program)
install(FILES com.yuos.Office.desktop DESTINATION share/applications)
install(FILES com.yuos.Office.svg DESTINATION share/icons/hicolor/scalable/apps)
