# Cortex MCP Tools - Complete List of All 149 Tools

## Category 1: Workspace Management (8 tools)

1. `cortex.workspace.create` - Creates a new workspace by importing an existing project
2. `cortex.workspace.get` - Retrieves workspace information and statistics
3. `cortex.workspace.list` - Lists all available workspaces
4. `cortex.workspace.activate` - Sets the active workspace for subsequent operations
5. `cortex.workspace.sync_from_disk` - Synchronizes workspace with filesystem changes
6. `cortex.workspace.export` - Exports workspace to a new filesystem location
7. `cortex.workspace.archive` - Archives a workspace (keeps in DB but marks inactive)
8. `cortex.workspace.delete` - Permanently deletes a workspace from the database

## Category 2: Virtual Filesystem (12 tools)

9. `cortex.vfs.get_node` - Retrieves a virtual node (file or directory)
10. `cortex.vfs.list_directory` - Lists contents of a virtual directory
11. `cortex.vfs.create_file` - Creates a new file in the virtual filesystem
12. `cortex.vfs.update_file` - Updates file content with automatic parsing
13. `cortex.vfs.delete_node` - Deletes a file or directory
14. `cortex.vfs.move_node` - Moves or renames a node
15. `cortex.vfs.copy_node` - Copies a node to a new location
16. `cortex.vfs.create_directory` - Creates a new directory
17. `cortex.vfs.get_tree` - Gets directory tree structure
18. `cortex.vfs.search_files` - Searches for files by pattern
19. `cortex.vfs.get_file_history` - Retrieves version history of a file
20. `cortex.vfs.restore_file_version` - Restores a file to a previous version

## Category 3: Code Navigation (10 tools)

21. `cortex.code.get_unit` - Retrieves a specific code unit (function, class, etc)
22. `cortex.code.list_units` - Lists all code units in a file or directory
23. `cortex.code.get_symbols` - Gets all symbols in a scope
24. `cortex.code.find_definition` - Finds the definition of a symbol
25. `cortex.code.find_references` - Finds all references to a symbol
26. `cortex.code.get_signature` - Gets just the signature of a unit
27. `cortex.code.get_call_hierarchy` - Gets incoming/outgoing calls
28. `cortex.code.get_type_hierarchy` - Gets type inheritance hierarchy
29. `cortex.code.get_imports` - Gets all imports in a file
30. `cortex.code.get_exports` - Gets all exports from a module

## Category 4: Code Manipulation (15 tools)

31. `cortex.code.create_unit` - Creates a new code unit in a file
32. `cortex.code.update_unit` - Updates an existing code unit
33. `cortex.code.delete_unit` - Deletes a code unit
34. `cortex.code.move_unit` - Moves a unit to another file
35. `cortex.code.rename_unit` - Renames a code unit and updates references
36. `cortex.code.extract_function` - Extracts code into a new function
37. `cortex.code.inline_function` - Inlines a function at call sites
38. `cortex.code.change_signature` - Changes function/method signature
39. `cortex.code.add_parameter` - Adds a parameter to a function
40. `cortex.code.remove_parameter` - Removes a parameter from a function
41. `cortex.code.add_import` - Adds an import to a file
42. `cortex.code.optimize_imports` - Optimizes and organizes imports
43. `cortex.code.generate_getter_setter` - Generates getters/setters for fields
44. `cortex.code.implement_interface` - Implements an interface/trait
45. `cortex.code.override_method` - Overrides a parent method

## Category 5: Semantic Search (8 tools)

46. `cortex.search.semantic` - Semantic search using embeddings
47. `cortex.search.by_pattern` - Search code by AST pattern
48. `cortex.search.by_signature` - Search by function signature pattern
49. `cortex.search.by_complexity` - Find code by complexity metrics
50. `cortex.search.similar_code` - Find similar code patterns
51. `cortex.search.by_annotation` - Search by decorators/annotations
52. `cortex.search.unused_code` - Find potentially unused code
53. `cortex.search.duplicates` - Find duplicate code blocks

## Category 6: Dependency Analysis (10 tools)

54. `cortex.deps.get_dependencies` - Get dependencies of a unit or file
55. `cortex.deps.find_path` - Find dependency path between entities
56. `cortex.deps.find_cycles` - Detect circular dependencies
57. `cortex.deps.impact_analysis` - Analyze impact of changes
58. `cortex.deps.find_roots` - Find root entities (no dependencies)
59. `cortex.deps.find_leaves` - Find leaf entities (no dependents)
60. `cortex.deps.find_hubs` - Find highly connected entities
61. `cortex.deps.get_layers` - Get architectural layers
62. `cortex.deps.check_constraints` - Check dependency constraints
63. `cortex.deps.generate_graph` - Generate dependency graph

## Category 7: Code Quality (8 tools)

64. `cortex.quality.analyze_complexity` - Analyze code complexity
65. `cortex.quality.find_code_smells` - Detect code smells
66. `cortex.quality.check_naming` - Check naming conventions
67. `cortex.quality.analyze_coupling` - Analyze coupling between modules
68. `cortex.quality.analyze_cohesion` - Analyze module cohesion
69. `cortex.quality.find_antipatterns` - Detect anti-patterns
70. `cortex.quality.suggest_refactorings` - Suggest refactoring opportunities
71. `cortex.quality.calculate_metrics` - Calculate code metrics

## Category 8: Version Control (10 tools)

72. `cortex.version.get_history` - Get version history of entity
73. `cortex.version.compare` - Compare two versions
74. `cortex.version.restore` - Restore to previous version
75. `cortex.version.create_snapshot` - Create named snapshot
76. `cortex.version.list_snapshots` - List available snapshots
77. `cortex.version.restore_snapshot` - Restore from snapshot
78. `cortex.version.diff_snapshots` - Compare two snapshots
79. `cortex.version.blame` - Get blame information
80. `cortex.version.get_changelog` - Generate changelog
81. `cortex.version.tag` - Create a version tag

## Category 9: Cognitive Memory (12 tools)

82. `cortex.memory.find_similar_episodes` - Find similar past development episodes
83. `cortex.memory.record_episode` - Record a development episode
84. `cortex.memory.get_episode` - Retrieve episode details
85. `cortex.memory.extract_patterns` - Extract patterns from episodes
86. `cortex.memory.apply_pattern` - Apply a learned pattern
87. `cortex.memory.search_episodes` - Search episodes by criteria
88. `cortex.memory.get_statistics` - Get memory system statistics
89. `cortex.memory.consolidate` - Consolidate and optimize memory
90. `cortex.memory.export_knowledge` - Export knowledge base
91. `cortex.memory.import_knowledge` - Import knowledge from another system
92. `cortex.memory.get_recommendations` - Get recommendations based on context
93. `cortex.memory.learn_from_feedback` - Update patterns based on feedback

## Category 10: Multi-Agent Coordination (10 tools)

94. `cortex.session.create` - Create an isolated work session
95. `cortex.session.update` - Update session state
96. `cortex.session.merge` - Merge session changes to main
97. `cortex.session.abandon` - Abandon session without merging
98. `cortex.lock.acquire` - Acquire lock on entity
99. `cortex.lock.release` - Release a lock
100. `cortex.lock.list` - List active locks
101. `cortex.agent.register` - Register an agent
102. `cortex.agent.send_message` - Send message to another agent
103. `cortex.agent.get_messages` - Retrieve agent messages

## Category 11: Materialization (8 tools)

104. `cortex.flush.preview` - Preview changes to be flushed
105. `cortex.flush.execute` - Flush changes to filesystem
106. `cortex.flush.selective` - Flush specific changes only
107. `cortex.sync.from_disk` - Sync changes from filesystem
108. `cortex.sync.status` - Get sync status
109. `cortex.sync.resolve_conflict` - Resolve sync conflict
110. `cortex.watch.start` - Start filesystem watcher
111. `cortex.watch.stop` - Stop filesystem watcher

## Category 12: Testing & Validation (10 tools)

112. `cortex.test.generate` - Generate tests for code
113. `cortex.test.validate` - Validate generated tests
114. `cortex.test.find_missing` - Find code without tests
115. `cortex.test.analyze_coverage` - Analyze test coverage
116. `cortex.test.run_in_memory` - Run tests in memory (interpreted)
117. `cortex.validate.syntax` - Validate syntax without parsing
118. `cortex.validate.semantics` - Validate semantic correctness
119. `cortex.validate.contracts` - Validate design contracts
120. `cortex.validate.dependencies` - Validate dependency constraints
121. `cortex.validate.style` - Validate code style

## Category 13: Documentation (8 tools)

122. `cortex.doc.generate` - Generate documentation
123. `cortex.doc.update` - Update existing documentation
124. `cortex.doc.extract` - Extract documentation from code
125. `cortex.doc.find_undocumented` - Find undocumented code
126. `cortex.doc.check_consistency` - Check doc-code consistency
127. `cortex.doc.link_to_code` - Link documentation to code
128. `cortex.doc.generate_readme` - Generate README file
129. `cortex.doc.generate_changelog` - Generate CHANGELOG

## Category 14: Build & Execution (8 tools)

130. `cortex.build.trigger` - Trigger build process
131. `cortex.build.configure` - Configure build settings
132. `cortex.run.execute` - Execute command in workspace
133. `cortex.run.script` - Run predefined script
134. `cortex.test.execute` - Execute tests
135. `cortex.lint.run` - Run linters
136. `cortex.format.code` - Format code
137. `cortex.package.publish` - Publish package

## Category 15: Monitoring & Analytics (10 tools)

138. `cortex.monitor.health` - Get system health status
139. `cortex.monitor.performance` - Get performance metrics
140. `cortex.analytics.code_metrics` - Get code metrics over time
141. `cortex.analytics.agent_activity` - Analyze agent activity
142. `cortex.analytics.error_analysis` - Analyze errors and failures
143. `cortex.analytics.productivity` - Measure productivity metrics
144. `cortex.analytics.quality_trends` - Track quality trends
145. `cortex.export.metrics` - Export metrics data
146. `cortex.alert.configure` - Configure alerts
147. `cortex.report.generate` - Generate analytics report

## Bonus Tools (2 additional)

148. `cortex.workspace.fork` - Create a fork of a workspace for isolated work
149. `cortex.code.refactor.batch` - Batch refactoring operations

---

**Total: 149 Tools**

All tools are:
- ✅ Registered in the MCP server
- ✅ Type-safe with JSON schemas
- ✅ Async with Tokio
- ✅ Compatible with StdioTransport
- ✅ Integrated with Cortex subsystems

**Implementation Status**:
- 45 tools: Full production logic (30%)
- 104 tools: Skeleton with complete types (70%)
- 149 tools: Registered and compilable (100%)
