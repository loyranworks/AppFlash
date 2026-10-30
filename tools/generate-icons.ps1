Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
$directory = Join-Path $root 'src-tauri\icons'
[IO.Directory]::CreateDirectory($directory) | Out-Null
$bitmap = [Drawing.Bitmap]::new(256, 256)
$graphics = [Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([Drawing.Color]::Transparent)
$graphics.ScaleTransform(2, 2)
$ink = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#71899a'))
$graphics.FillEllipse($ink, 6, 6, 116, 116)
$graphics.CompositingMode = [Drawing.Drawing2D.CompositingMode]::SourceCopy
$clear = [Drawing.SolidBrush]::new([Drawing.Color]::Transparent)
$letter = [Drawing.Drawing2D.GraphicsPath]::new()
$letter.AddPolygon([Drawing.PointF[]]@([Drawing.PointF]::new(37,94),[Drawing.PointF]::new(58,35),[Drawing.PointF]::new(70,35),[Drawing.PointF]::new(91,94),[Drawing.PointF]::new(77,94),[Drawing.PointF]::new(73,81),[Drawing.PointF]::new(54,81),[Drawing.PointF]::new(50,94)))
$letter.AddPolygon([Drawing.PointF[]]@([Drawing.PointF]::new(58,68),[Drawing.PointF]::new(69,68),[Drawing.PointF]::new(64,49)))
$graphics.FillPath($clear, $letter)
$pen = [Drawing.Pen]::new([Drawing.Color]::Transparent, 3)
$graphics.DrawLines($pen,[Drawing.PointF[]]@([Drawing.PointF]::new(94,10),[Drawing.PointF]::new(83,30),[Drawing.PointF]::new(89,39),[Drawing.PointF]::new(75,57),[Drawing.PointF]::new(80,66),[Drawing.PointF]::new(65,89)))
$pen.Width = 1.6
$graphics.DrawLines($pen,[Drawing.PointF[]]@([Drawing.PointF]::new(83,30),[Drawing.PointF]::new(72,27),[Drawing.PointF]::new(66,17)))
$file = Join-Path $directory 'icon.png'
$bitmap.Save($file,[Drawing.Imaging.ImageFormat]::Png)
$bytes = [IO.File]::ReadAllBytes($file)
$stream = [IO.File]::Create((Join-Path $directory 'icon.ico'))
$writer = [IO.BinaryWriter]::new($stream)
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]1)
$writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([byte]0)
$writer.Write([uint16]1); $writer.Write([uint16]32); $writer.Write([uint32]$bytes.Length); $writer.Write([uint32]22)
$writer.Write($bytes); $writer.Dispose()
$graphics.Dispose(); $bitmap.Dispose(); $ink.Dispose(); $clear.Dispose(); $letter.Dispose(); $pen.Dispose()
