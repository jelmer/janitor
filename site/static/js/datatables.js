/**
 * DataTables Integration for Janitor
 * Provides enhanced table functionality with sorting, pagination, and filtering
 */

(function(window, document, $) {
    'use strict';

    // Ensure Janitor namespace exists
    window.Janitor = window.Janitor || {};

    /**
     * DataTables Module
     */
    Janitor.DataTables = (function() {
        
        // Default configuration for all DataTables
        var defaultConfig = {
            pageLength: 50,
            lengthMenu: [[50, 200, 500, 1000, -1], [50, 200, 500, 1000, "All"]],
            pagingType: "full_numbers",
            searching: true,
            ordering: true,
            info: true,
            autoWidth: false,
            responsive: true,
            language: {
                search: "Filter:",
                lengthMenu: "Show _MENU_ entries per page",
                info: "Showing _START_ to _END_ of _TOTAL_ entries",
                infoEmpty: "No entries found",
                infoFiltered: "(filtered from _MAX_ total entries)",
                paginate: {
                    first: "First",
                    last: "Last",
                    next: "Next",
                    previous: "Previous"
                }
            },
            dom: '<"top"lf>rt<"bottom"ip><"clear">'
        };

        // Specialized configurations for different table types
        var tableConfigs = {
            resultCodes: {
                order: [[1, 'desc']], // Sort by count column descending
                columnDefs: [
                    {
                        targets: [1], // Count column
                        type: 'num',
                        render: function(data, type, row) {
                            if (type === 'display') {
                                return parseInt(data).toLocaleString();
                            }
                            return data;
                        }
                    }
                ]
            },
            failureStages: {
                order: [[1, 'desc']], // Sort by count column descending
                columnDefs: [
                    {
                        targets: [1], // Count column
                        type: 'num',
                        render: function(data, type, row) {
                            if (type === 'display') {
                                return parseInt(data).toLocaleString();
                            }
                            return data;
                        }
                    }
                ]
            },
            runHistory: {
                order: [[0, 'desc']], // Sort by date column descending
                columnDefs: [
                    {
                        targets: 'date-column',
                        type: 'date',
                        render: function(data, type, row) {
                            if (type === 'display' && data) {
                                var date = new Date(data);
                                return date.toLocaleString();
                            }
                            return data;
                        }
                    },
                    {
                        targets: 'duration-column',
                        type: 'num',
                        render: function(data, type, row) {
                            if (type === 'display' && data && window.Janitor && window.Janitor.Utils) {
                                return window.Janitor.Utils.formatDuration(data);
                            }
                            return data;
                        }
                    }
                ]
            },
            packageList: {
                order: [[0, 'asc']], // Sort by package name ascending
                columnDefs: [
                    {
                        targets: 'no-sort',
                        orderable: false
                    }
                ]
            },
            publishHistory: {
                order: [[0, 'desc']], // Sort by date descending
                columnDefs: [
                    {
                        targets: 'date-column',
                        type: 'date'
                    }
                ]
            },
            queue: {
                order: [[2, 'desc']], // Sort by priority descending
                columnDefs: [
                    {
                        targets: [2], // Priority column
                        type: 'num'
                    }
                ]
            }
        };

        /**
         * Initialize a DataTable with appropriate configuration
         */
        function initTable(selector, options) {
            var $table = $(selector);
            if ($table.length === 0) {
                console.warn('DataTable selector not found:', selector);
                return null;
            }

            // Determine table type from data attributes or options
            var tableType = options && options.type;
            if (!tableType) {
                tableType = $table.data('table-type') || 
                           $table.attr('class').split(' ').find(function(cls) {
                               return tableConfigs.hasOwnProperty(cls.replace('table-', ''));
                           });
            }

            // Build configuration
            var config = $.extend(true, {}, defaultConfig);
            if (tableType && tableConfigs[tableType]) {
                config = $.extend(true, config, tableConfigs[tableType]);
            }
            if (options) {
                config = $.extend(true, config, options);
            }

            // Apply any custom column definitions from data attributes
            $table.find('th[data-type]').each(function(index) {
                var $th = $(this);
                var type = $th.data('type');
                var sortable = $th.data('sortable');
                
                config.columnDefs = config.columnDefs || [];
                config.columnDefs.push({
                    targets: [index],
                    type: type,
                    orderable: sortable !== false
                });
            });

            try {
                var dataTable = $table.DataTable(config);
                
                // Store reference for later use
                $table.data('datatable', dataTable);
                
                // Add custom event handlers
                if (options && options.onRowClick) {
                    $table.on('click', 'tbody tr', function() {
                        var data = dataTable.row(this).data();
                        options.onRowClick(data, this);
                    });
                }

                if (options && options.onDraw) {
                    dataTable.on('draw', options.onDraw);
                }

                return dataTable;
            } catch (error) {
                console.error('Failed to initialize DataTable:', error);
                return null;
            }
        }

        /**
         * Initialize result codes table
         */
        function initResultCodesTable(selector, options) {
            var config = $.extend({}, {
                type: 'resultCodes',
                onRowClick: function(data, row) {
                    // Navigate to result code detail page
                    var code = $(row).find('td:first a').attr('href');
                    if (code) {
                        window.location.href = code;
                    }
                }
            }, options);

            return initTable(selector, config);
        }

        /**
         * Initialize failure stages table
         */
        function initFailureStagesTable(selector, options) {
            var config = $.extend({}, {
                type: 'failureStages',
                onRowClick: function(data, row) {
                    var link = $(row).find('td:first a').attr('href');
                    if (link) {
                        window.location.href = link;
                    }
                }
            }, options);

            return initTable(selector, config);
        }

        /**
         * Initialize run history table
         */
        function initRunHistoryTable(selector, options) {
            var config = $.extend({}, {
                type: 'runHistory'
            }, options);

            return initTable(selector, config);
        }

        /**
         * Initialize package list table
         */
        function initPackageListTable(selector, options) {
            var config = $.extend({}, {
                type: 'packageList'
            }, options);

            return initTable(selector, config);
        }

        /**
         * Initialize publish history table
         */
        function initPublishHistoryTable(selector, options) {
            var config = $.extend({}, {
                type: 'publishHistory'
            }, options);

            return initTable(selector, config);
        }

        /**
         * Initialize queue table with real-time updates
         */
        function initQueueTable(selector, options) {
            var config = $.extend({}, {
                type: 'queue',
                onDraw: function() {
                    // Re-attach event handlers after redraw
                    attachQueueEventHandlers();
                }
            }, options);

            var dataTable = initTable(selector, config);
            
            // Set up real-time updates via WebSocket
            if (window.Janitor && window.Janitor.WebSocket) {
                window.Janitor.WebSocket.registerHandler('queue_update', function(data) {
                    updateQueueTable(dataTable, data);
                });
            }

            return dataTable;
        }

        /**
         * Update queue table with real-time data
         */
        function updateQueueTable(dataTable, queueData) {
            if (!dataTable || !queueData) return;

            try {
                // Clear existing data
                dataTable.clear();
                
                // Add new rows
                if (Array.isArray(queueData)) {
                    queueData.forEach(function(item) {
                        var rowData = [
                            item.codebase || '',
                            item.campaign || '',
                            item.priority || 0,
                            item.status || '',
                            item.worker || '',
                            item.started_at || ''
                        ];
                        dataTable.row.add(rowData);
                    });
                }
                
                // Redraw table
                dataTable.draw();
            } catch (error) {
                console.error('Failed to update queue table:', error);
            }
        }

        /**
         * Attach event handlers for queue management
         */
        function attachQueueEventHandlers() {
            // Kill job buttons
            $('.kill-job-btn').off('click').on('click', function(e) {
                e.preventDefault();
                var runId = $(this).data('run-id');
                var $btn = $(this);
                
                if (confirm('Are you sure you want to kill this job?')) {
                    $btn.prop('disabled', true).text('Killing...');
                    
                    Janitor.Ajax.post('/api/runs/' + runId + '/kill')
                        .done(function() {
                            Janitor.Utils.showNotification('Job killed successfully', 'success');
                            // Row will be updated via WebSocket
                        })
                        .fail(function() {
                            Janitor.Utils.showNotification('Failed to kill job', 'error');
                            $btn.prop('disabled', false).text('Kill');
                        });
                }
            });
        }

        /**
         * Initialize all tables automatically based on CSS classes
         */
        function initAllTables() {
            // Result codes tables
            $('.table-result-codes, .result-codes-table').each(function() {
                initResultCodesTable(this);
            });

            // Failure stages tables
            $('.table-failure-stages, .failure-stages-table').each(function() {
                initFailureStagesTable(this);
            });

            // Run history tables
            $('.table-run-history, .run-history-table').each(function() {
                initRunHistoryTable(this);
            });

            // Package list tables
            $('.table-package-list, .package-list-table').each(function() {
                initPackageListTable(this);
            });

            // Publish history tables
            $('.table-publish-history, .publish-history-table').each(function() {
                initPublishHistoryTable(this);
            });

            // Queue tables
            $('.table-queue, .queue-table').each(function() {
                initQueueTable(this);
            });

            // Generic DataTables (using data attributes)
            $('table[data-datatable="true"]').each(function() {
                var $table = $(this);
                var config = {};
                
                // Read configuration from data attributes
                if ($table.data('page-length')) {
                    config.pageLength = parseInt($table.data('page-length'));
                }
                if ($table.data('order')) {
                    config.order = $table.data('order');
                }
                
                initTable(this, config);
            });
        }

        /**
         * Refresh a specific DataTable
         */
        function refreshTable(selector) {
            var $table = $(selector);
            var dataTable = $table.data('datatable');
            
            if (dataTable) {
                dataTable.ajax.reload(null, false); // Don't reset paging
            }
        }

        /**
         * Get DataTable instance
         */
        function getTable(selector) {
            return $(selector).data('datatable');
        }

        // Initialize when DOM is ready
        $(document).ready(function() {
            initAllTables();
        });

        // Public API
        return {
            initTable: initTable,
            initResultCodesTable: initResultCodesTable,
            initFailureStagesTable: initFailureStagesTable,
            initRunHistoryTable: initRunHistoryTable,
            initPackageListTable: initPackageListTable,
            initPublishHistoryTable: initPublishHistoryTable,
            initQueueTable: initQueueTable,
            initAllTables: initAllTables,
            refreshTable: refreshTable,
            getTable: getTable,
            defaultConfig: defaultConfig,
            tableConfigs: tableConfigs
        };
    })();

})(window, document, jQuery);