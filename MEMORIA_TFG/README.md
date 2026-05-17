# Plantilla LaTeX para TFG UIE

**Versión de la plantilla:** 1.0

Plantilla institucional de la **Universidad Intercontinental de la Empresa (UIE)** para la elaboración de memorias de **Trabajo Fin de Grado (TFG)** de la **Facultad de Ingeniería y Tecnología Empresarial**.

La plantilla está diseñada para su uso en **Overleaf**, con una estructura modular que facilita la edición, el mantenimiento y la reutilización del proyecto.

## 1. Objetivo de la plantilla

Esta plantilla proporciona una base estable para la redacción de la memoria académica del TFG, manteniendo una estructura formal coherente con el formato institucional y facilitando la organización del documento en LaTeX.

La plantilla ha sido diseñada con los siguientes criterios:

- compatibilidad con **Overleaf**;
- código LaTeX **portable** y basado en soluciones ampliamente utilizadas;
- organización modular en varios ficheros `.tex`;
- separación entre **configuración** y **contenido**;
- facilidad de edición para estudiantes y directores;
- soporte para bibliografía en formato **APA** mediante `biblatex`.

## 2. Compilación recomendada

La plantilla está preparada para su uso en **Overleaf** y se recomienda compilarla con la siguiente configuración:

- **Compiler**: `pdfLaTeX`
- **Bibliography tool**: `Biber`

## 3. Estructura del proyecto

```text
main.tex
bibliografia.bib
README.md
imagenes/
configuracion/
  paquetes.tex
  datos.tex
  estilos.tex
  comandos.tex
preliminares/
  resumen.tex
  abstract.tex
  dedicatoria.tex
capitulos/
  01_introduccion.tex
  02_marco_teorico.tex
  03_analisis_situacion.tex
  04_desarrollo.tex
  05_resultados.tex
  06_conclusiones.tex
anexos/
  anexo_01_iag.tex
  anexo_02.tex
  anexo_03.tex
```

## 4. Función de cada archivo

### `main.tex`

Es el archivo principal del proyecto.  
Contiene la estructura general del documento y llama al resto de archivos mediante `\input`.

### `bibliografia.bib`

Archivo de referencias bibliográficas en formato BibTeX/Biber.

### `configuracion/paquetes.tex`

Carga todos los paquetes necesarios y configura la bibliografía y los hipervínculos.

### `configuracion/datos.tex`

Contiene los datos editables de la portada y algunos parámetros generales de activación o desactivación de elementos opcionales.

### `configuracion/estilos.tex`

Define el formato general del documento:

- interlineado,
- párrafos,
- cabeceras y pies,
- títulos,
- índices.

### `configuracion/comandos.tex`

Incluye los comandos auxiliares que controlan la portada, los preliminares y la bibliografía.

### `preliminares/`

Contiene el contenido editable de:

- resumen,
- abstract,
- dedicatoria (si se activa su inclusión en la plantilla).

### `capitulos/`

Contiene los capítulos principales de la memoria.

### `anexos/`

Contiene los anexos del documento.

### `imagenes/`

Carpeta para los recursos gráficos de la plantilla, en particular:

- `logo_uie.png`
- `sello_uie.png`
- `cc_by_nc_nd.png`

## 5. Qué debe editar normalmente el estudiante

En un uso ordinario, el estudiante debería editar únicamente:

- `configuracion/datos.tex`
- `preliminares/resumen.tex`
- `preliminares/abstract.tex`
- `preliminares/dedicatoria.tex` (solo si se activa la dedicatoria)
- los archivos dentro de `capitulos/`
- los archivos dentro de `anexos/`
- `bibliografia.bib`

## 6. Qué no conviene modificar salvo necesidad

Se recomienda no modificar sin criterio claro:

- `configuracion/paquetes.tex`
- `configuracion/estilos.tex`
- `configuracion/comandos.tex`

Estos archivos controlan la maquetación global de la plantilla. Cambios en ellos pueden alterar la portada, la numeración, los índices o el estilo institucional del documento.

## 7. Portada y licencia

La portada incluye, cuando se activa, un bloque de licencia Creative Commons.  
Este comportamiento se controla en el archivo:

```latex
configuracion/datos.tex
```

mediante el booleano:

```latex
\booltrue{publicacionabierta}
```

o, alternativamente,

```latex
\boolfalse{publicacionabierta}
```

Use `\booltrue{publicacionabierta}` solo en el caso de autorizar la publicación en el repositorio abierto de la UIE.

## 8. Gestión de directores

La plantilla admite uno o dos directores.

En `configuracion/datos.tex`:

```latex
\newcommand{\directorUno}{Apellidos y Nombres del Director}
\newcommand{\directorDos}{}
```

- Si solo hay un director, deje `\directorDos` vacío.
- Si hay dos directores, escriba el segundo nombre en `\directorDos`.

## 9. Dedicatoria opcional

La plantilla permite incluir o excluir la dedicatoria de forma explícita.

Este comportamiento se controla en:

```latex
configuracion/datos.tex
```

mediante el booleano:

```latex
\booltrue{incluirdedicatoria}
```

o, alternativamente,

```latex
\boolfalse{incluirdedicatoria}
```

- Si se usa `\booltrue{incluirdedicatoria}`, la dedicatoria se incluye en los preliminares.
- Si se usa `\boolfalse{incluirdedicatoria}`, la dedicatoria no aparece en el documento.

El contenido de la dedicatoria debe escribirse en:

```text
preliminares/dedicatoria.tex
```

## 10. Bibliografía

La plantilla utiliza:

- `biblatex`
- estilo `apa`
- backend `biber`

Las referencias deben añadirse en `bibliografia.bib`.

Ejemplo mínimo:

```bibtex
@article{garcia2024ejemplo,
  author       = {García, Ana and López, Carlos},
  title        = {Título del artículo},
  journaltitle = {Nombre de la revista},
  year         = {2024},
  volume       = {10},
  number       = {2},
  pages        = {15--30},
  doi          = {10.0000/ejemplo.2024.001}
}
```

En el campo `author`, varios autores se separan mediante la palabra `and`.

Para citar en el texto, pueden utilizarse comandos habituales como:

```latex
\textcite{garcia2024ejemplo}
\parencite{garcia2024ejemplo}
```

## 11. Imágenes institucionales

Para que la plantilla reproduzca correctamente la portada y el encabezado, deben existir en la carpeta `imagenes/` los siguientes archivos:

```text
logo_uie.png
sello_uie.png
cc_by_nc_nd.png
```

Si alguno de ellos no está presente, la plantilla seguirá compilando, pero mostrará un marcador de posición en lugar de la imagen correspondiente.

## 12. Organización del trabajo

La estructura modular permite trabajar de forma ordenada:

- un archivo para cada capítulo;
- un archivo para cada anexo;
- separación clara entre contenido y maquetación.

Este diseño facilita:

- la revisión por partes,
- la corrección por el director,
- la reutilización de la plantilla,
- y el mantenimiento del proyecto en documentos largos.

## 13. Recomendaciones de uso

- Compilar siempre el proyecto desde `main.tex`.
- Añadir las referencias en `bibliografia.bib` desde el inicio del trabajo.
- Mantener nombres de archivo simples, sin espacios ni caracteres especiales.
- Guardar todas las figuras en la carpeta `imagenes/`.
- Evitar modificar la configuración global salvo necesidad justificada.