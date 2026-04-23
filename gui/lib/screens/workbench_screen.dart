import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as path;
import 'package:provider/provider.dart';

import '../state/workbench_state.dart';
import '../widgets/log_panel.dart';
import '../widgets/source_panel.dart';
import '../widgets/toolbar.dart';

class WorkbenchScreen extends StatelessWidget {
  const WorkbenchScreen({super.key});

  Future<void> _pickSample(BuildContext context, SourceKind kind) async {
    final state = context.read<WorkbenchState>();
    final cwd = Directory.current.path;
    final fileName = kind == SourceKind.tai ? 'cli/sample.tai' : 'cli/sample.meng';
    final file = File(path.join(cwd, fileName));
    if (await file.exists()) {
      state.openPath(file.path, kind);
      return;
    }
    state.recordInfo('Sample file not found: $fileName');
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WorkbenchState>();

    return Scaffold(
      appBar: AppBar(
        title: const Text('Tailang Workbench'),
        centerTitle: false,
        actions: [
          TextButton(
            onPressed: () => _pickSample(context, SourceKind.tai),
            child: const Text('Open sample.tai'),
          ),
          TextButton(
            onPressed: () => _pickSample(context, SourceKind.meng),
            child: const Text('Open sample.meng'),
          ),
        ],
      ),
      body: Column(
        children: [
          const WorkbenchToolbar(),
          Expanded(
            child: Row(
              children: [
                Expanded(
                  child: SourcePanel(
                    title: '.meng Editor',
                    content: state.currentSourceKind == SourceKind.meng
                        ? state.currentSourceText
                        : '',
                    selected: state.currentSourceKind == SourceKind.meng,
                  ),
                ),
                const VerticalDivider(width: 1),
                Expanded(
                  child: SourcePanel(
                    title: '.tai Preview',
                    content: state.currentTaiPreview,
                    selected: state.currentSourceKind == SourceKind.tai,
                  ),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          const SizedBox(
            height: 180,
            child: LogPanel(),
          ),
        ],
      ),
    );
  }
}
