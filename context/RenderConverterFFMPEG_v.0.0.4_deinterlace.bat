@ECHO OFF
title Render FFMPEG Converter
mode 70, 15
color 3F

:config
cls
ECHO ----------------SCRIPT-FFMPEG(@laucalixto)----------------
ECHO -----------CONFIGURACAO PARA EXPORTACAO-----------
ECHO.
SET LARGURA=1920
SET ALTURA=1080
SET QUALIDADE=19
SET QUALIDADEAUDIO=128
ECHO "Defina a largura:"
SET /p LARGURA=">"
ECHO "Defina a altura:"
SET /p ALTURA=">"
ECHO "Defina a qualidade (0 lossles at 51 worst)"
ECHO "Default 19:"
SET /p QUALIDADE=">"
ECHO "Defina a qualidade de audio (select 96, 128, 192, 320 /128 is default)"
SET /p QUALIDADEAUDIO=">"


:confirmacao
cls
ECHO -----------VERIFICAR ENTRADAS-----------
echo "Resolucao:" %LARGURA% x %ALTURA% 
echo "Qualidade:" %QUALIDADE%
echo "Qualidade audio:" %QUALIDADEAUDIO%kbps
echo "Confirmar?"
echo
echo [1] "Configurar novamente?"
echo [2] "Iniciar render"
echo [3] "Sair"
set/p menuConfirmacao=">"
if %menuConfirmacao% equ 1 goto config
if %menuConfirmacao% equ 2 goto render
if %menuConfirmacao% equ 3 goto fim



:render
cls
if not exist "%cd%\low" mkdir "%cd%\low"
for %%a in ("*.*") do ffmpeg -i "%%a" -c:v libx264 -crf %QUALIDADE% -vf yadif,scale=%LARGURA%:%ALTURA% -pix_fmt yuv420p -movflags faststart -threads 2 -b:a %QUALIDADEAUDIO%k "low\%%~na.mp4"

Call: complete



:complete
cls
ECHO "render complete"
Call: menuRetorno


:menuRetorno
cls
echo [1] "Configurar novamente?"
echo [2] "Sair"
set/p menu=">"

if %menu% equ 1 goto config
if %menu% equ 2 goto fim
if not %menu% == 1 if not %menu% == 2 goto config 





:fim
exit



