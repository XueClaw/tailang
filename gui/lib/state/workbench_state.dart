import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as path;

enum SourceKind { meng, tai }

enum TaskState { idle, running, success, failure }

class WorkbenchState extends ChangeNotifier {
  String currentPath = '';
  String currentDirectory = '';
  String currentSourceText = '';
  String currentTaiPreview = '';
  String lastOutputPath = '';
  SourceKind currentSourceKind = SourceKind.tai;
  TaskState precompileState = TaskState.idle;
  TaskState buildState = TaskState.idle;
  TaskState runState = TaskState.idle;
  final List<String> logs = <String>['Tailang GUI workbench ready.'];

  bool get hasSelection => currentPath.isNotEmpty;

  String get lastLog => logs.isEmpty ? '' : logs.last;

  void openPath(String pathValue, SourceKind kind) {
    currentPath = pathValue;
    currentDirectory = path.dirname(pathValue);
    currentSourceKind = kind;
    lastOutputPath = kind == SourceKind.tai
        ? path.join(currentDirectory, '${path.basenameWithoutExtension(pathValue)}.exe')
        : '';
    _appendLog('Opened $pathValue');
    _loadFilePreview();
    notifyListeners();
  }

  void recordInfo(String message) {
    _appendLog(message);
    notifyListeners();
  }

  Future<void> precompile() async {
    if (!hasSelection || currentSourceKind != SourceKind.meng) {
      _appendLog('Precompile requires a selected .meng file.');
      precompileState = TaskState.failure;
      notifyListeners();
      return;
    }

    final outputTai =
        path.join(currentDirectory, '${path.basenameWithoutExtension(currentPath)}.tai');
    precompileState = TaskState.running;
    _appendLog('Running: meng precompile ${path.basename(currentPath)} -o ${path.basename(outputTai)}');
    notifyListeners();

    final result = await _runMengCommand(<String>[
      'precompile',
      currentPath,
      '-o',
      outputTai,
    ]);
    if (result.exitCode == 0) {
      precompileState = TaskState.success;
      currentTaiPreview = await _safeRead(outputTai);
      _appendLog('Precompile succeeded: $outputTai');
    } else {
      precompileState = TaskState.failure;
      _appendLog('Precompile failed.');
    }
    notifyListeners();
  }

  Future<void> build() async {
    if (!hasSelection) {
      _appendLog('Build requires a selected .tai or .meng file.');
      buildState = TaskState.failure;
      notifyListeners();
      return;
    }

    final outputPath =
        path.join(currentDirectory, '${path.basenameWithoutExtension(currentPath)}.exe');
    lastOutputPath = outputPath;
    buildState = TaskState.running;
    _appendLog('Running: meng build ${path.basename(currentPath)} -o ${path.basename(outputPath)}');
    notifyListeners();

    final result = await _runMengCommand(<String>[
      'build',
      currentPath,
      '-o',
      outputPath,
    ]);
    if (result.exitCode == 0) {
      buildState = TaskState.success;
      _appendLog('Build succeeded: $outputPath');
    } else {
      buildState = TaskState.failure;
      _appendLog('Build failed.');
    }
    notifyListeners();
  }

  Future<void> run() async {
    if (!hasSelection) {
      _appendLog('Run requires a selected .tai or .meng file.');
      runState = TaskState.failure;
      notifyListeners();
      return;
    }

    runState = TaskState.running;
    _appendLog('Running: meng run ${path.basename(currentPath)}');
    notifyListeners();

    final result = await _runMengCommand(<String>['run', currentPath]);
    if (result.exitCode == 0) {
      runState = TaskState.success;
      _appendLog('Run finished successfully.');
    } else {
      runState = TaskState.failure;
      _appendLog('Run failed.');
    }
    notifyListeners();
  }

  Future<void> _loadFilePreview() async {
    currentSourceText = await _safeRead(currentPath);
    if (currentSourceKind == SourceKind.tai) {
      currentTaiPreview = currentSourceText;
    }
  }

  Future<String> _safeRead(String filePath) async {
    try {
      return await File(filePath).readAsString();
    } catch (_) {
      return '';
    }
  }

  Future<ProcessResult> _runMengCommand(List<String> args) async {
    final process = await Process.run(
      'go',
      <String>['run', '.', ...args],
      workingDirectory: _findCliDirectory(),
    );
    final stdoutText = process.stdout.toString().trim();
    final stderrText = process.stderr.toString().trim();
    if (stdoutText.isNotEmpty) {
      _appendLog(stdoutText);
    }
    if (stderrText.isNotEmpty) {
      _appendLog(stderrText);
    }
    return process;
  }

  String _findCliDirectory() {
    var dir = Directory.current;
    while (true) {
      final candidate = Directory(path.join(dir.path, 'cli'));
      if (candidate.existsSync()) {
        return candidate.path;
      }
      final parent = dir.parent;
      if (parent.path == dir.path) {
        return Directory.current.path;
      }
      dir = parent;
    }
  }

  void _appendLog(String message) {
    for (final line in message.split('\n')) {
      final trimmed = line.trimRight();
      if (trimmed.isNotEmpty) {
        logs.add(trimmed);
      }
    }
  }
}
