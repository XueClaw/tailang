import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../state/workbench_state.dart';

class LogPanel extends StatelessWidget {
  const LogPanel({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WorkbenchState>();

    return Container(
      color: const Color(0xFF0F172A),
      padding: const EdgeInsets.all(16),
      width: double.infinity,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Logs',
            style: TextStyle(
              color: Colors.white,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 12),
          Expanded(
            child: ListView.builder(
              reverse: true,
              itemCount: state.logs.length,
              itemBuilder: (context, index) {
                final message = state.logs[state.logs.length - 1 - index];
                return Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Text(
                    message,
                    style: const TextStyle(
                      color: Color(0xFFBFDBFE),
                      fontFamily: 'Consolas',
                      fontSize: 13,
                    ),
                  ),
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
