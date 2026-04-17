import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/workbench_state.dart';

class WorkbenchToolbar extends StatelessWidget {
  const WorkbenchToolbar({super.key});

  Color _statusColor(TaskState state, ColorScheme scheme) {
    switch (state) {
      case TaskState.success:
        return scheme.primary;
      case TaskState.failure:
        return scheme.error;
      case TaskState.running:
        return scheme.tertiary;
      case TaskState.idle:
        return scheme.outline;
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WorkbenchState>();
    final scheme = Theme.of(context).colorScheme;

    return Material(
      color: Colors.white,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        child: Row(
          children: [
            FilledButton.tonal(
              onPressed:
                  state.currentSourceKind == SourceKind.meng && state.precompileState != TaskState.running
                      ? () => context.read<WorkbenchState>().precompile()
                      : null,
              child: const Text('Precompile'),
            ),
            const SizedBox(width: 12),
            FilledButton(
              onPressed: state.buildState != TaskState.running
                  ? () => context.read<WorkbenchState>().build()
                  : null,
              child: const Text('Build'),
            ),
            const SizedBox(width: 12),
            OutlinedButton(
              onPressed:
                  state.runState != TaskState.running ? () => context.read<WorkbenchState>().run() : null,
              child: const Text('Run'),
            ),
            const SizedBox(width: 16),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
              decoration: BoxDecoration(
                color: _statusColor(state.buildState, scheme).withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(999),
              ),
              child: Text(
                'Build: ${state.buildState.name}',
                style: TextStyle(
                  color: _statusColor(state.buildState, scheme),
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            const Spacer(),
            Text(
              state.currentPath.isEmpty ? 'No file selected' : state.currentPath,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ),
      ),
    );
  }
}
