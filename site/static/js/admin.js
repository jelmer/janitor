/**
 * Administrative Actions JavaScript for Janitor
 * Handles publishing, rescheduling, and other administrative operations
 */

(function(window, document, $) {
    'use strict';

    // Ensure Janitor namespace exists
    window.Janitor = window.Janitor || {};

    /**
     * Admin Module
     */
    Janitor.Admin = (function() {
        
        // Configuration
        var config = {
            endpoints: {
                publish: '/api/admin/publish',
                reschedule: '/api/admin/reschedule',
                kill: '/api/admin/kill',
                reprocess: '/api/admin/reprocess',
                scan: '/api/admin/scan',
                autopublish: '/api/admin/autopublish'
            },
            confirmations: {
                kill: 'Are you sure you want to kill this job?',
                reschedule: 'Are you sure you want to reschedule these items?',
                massReschedule: 'Are you sure you want to reschedule {count} items?',
                publish: 'Are you sure you want to publish this run?',
                reprocess: 'Are you sure you want to reprocess these logs?'
            }
        };

        /**
         * Initialize administrative functionality
         */
        function init() {
            attachEventHandlers();
            setupFormValidation();
            initializeButtons();
        }

        /**
         * Attach event handlers for admin actions
         */
        function attachEventHandlers() {
            // Publish actions
            $(document).on('click', '.publish-btn', handlePublishAction);
            $(document).on('click', '.publish-now-btn', handlePublishNow);
            $(document).on('click', '.create-fork-btn', handleCreateFork);
            $(document).on('click', '.push-btn', handlePush);
            $(document).on('click', '.create-mp-btn', handleCreateMergeProposal);

            // Reschedule actions
            $(document).on('click', '.reschedule-btn', handleReschedule);
            $(document).on('click', '.mass-reschedule-btn', handleMassReschedule);
            $(document).on('submit', '.reschedule-form', handleRescheduleForm);

            // Job management
            $(document).on('click', '.kill-job-btn', handleKillJob);
            $(document).on('click', '.reprocess-logs-btn', handleReprocessLogs);

            // Publishing management
            $(document).on('click', '.scan-btn', handleScan);
            $(document).on('click', '.autopublish-btn', handleAutopublish);

            // Form auto-submission
            Janitor.Forms.enableAutoSubmit('.auto-submit');

            // Checkbox selection for mass operations
            $(document).on('change', '.select-all-checkbox', handleSelectAll);
            $(document).on('change', '.item-checkbox', updateMassActionButtons);
        }

        /**
         * Handle publish action
         */
        function handlePublishAction(e) {
            e.preventDefault();
            var $btn = $(this);
            var action = $btn.data('action');
            var runId = $btn.data('run-id');
            var codebase = $btn.data('codebase');

            if (!confirm(config.confirmations.publish)) {
                return;
            }

            var originalText = $btn.text();
            $btn.prop('disabled', true).text('Publishing...');

            var data = {
                action: action,
                run_id: runId,
                codebase: codebase
            };

            Janitor.Ajax.post(config.endpoints.publish, data)
                .done(function(response) {
                    Janitor.Utils.showNotification('Publish action initiated successfully', 'success');
                    
                    // Update button state
                    if (response.status) {
                        $btn.removeClass('btn-primary').addClass('btn-success')
                            .text('Published').prop('disabled', true);
                    }
                    
                    // Update page content if provided
                    if (response.html) {
                        $btn.closest('.publish-actions').html(response.html);
                    }
                })
                .fail(function(xhr) {
                    var message = 'Publish action failed';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                    $btn.prop('disabled', false).text(originalText);
                });
        }

        /**
         * Handle individual publish actions
         */
        function handlePublishNow(e) {
            e.preventDefault();
            handlePublishAction.call(this, e);
        }

        function handleCreateFork(e) {
            e.preventDefault();
            handlePublishAction.call(this, e);
        }

        function handlePush(e) {
            e.preventDefault();
            handlePublishAction.call(this, e);
        }

        function handleCreateMergeProposal(e) {
            e.preventDefault();
            handlePublishAction.call(this, e);
        }

        /**
         * Handle reschedule action
         */
        function handleReschedule(e) {
            e.preventDefault();
            var $btn = $(this);
            var codebase = $btn.data('codebase');
            var campaign = $btn.data('campaign');

            if (!confirm(config.confirmations.reschedule)) {
                return;
            }

            var originalText = $btn.text();
            $btn.prop('disabled', true).text('Rescheduling...');

            var data = {
                codebase: codebase,
                campaign: campaign
            };

            Janitor.Ajax.post(config.endpoints.reschedule, data)
                .done(function(response) {
                    Janitor.Utils.showNotification('Item rescheduled successfully', 'success');
                    
                    if (response.position) {
                        $btn.parent().append(
                            '<span class="text-muted"> (Position: ' + response.position + ')</span>'
                        );
                    }
                })
                .fail(function(xhr) {
                    var message = 'Reschedule failed';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                })
                .always(function() {
                    $btn.prop('disabled', false).text(originalText);
                });
        }

        /**
         * Handle mass reschedule action
         */
        function handleMassReschedule(e) {
            e.preventDefault();
            var $form = $(this).closest('form');
            var selectedItems = $form.find('.item-checkbox:checked');
            
            if (selectedItems.length === 0) {
                Janitor.Utils.showNotification('No items selected', 'warning');
                return;
            }

            var message = config.confirmations.massReschedule.replace('{count}', selectedItems.length);
            if (!confirm(message)) {
                return;
            }

            e.preventDefault(); // Prevent default form submission
            
            var items = [];
            selectedItems.each(function() {
                items.push($(this).val());
            });

            var $btn = $(this);
            var originalText = $btn.text();
            $btn.prop('disabled', true).text('Rescheduling...');

            Janitor.Ajax.post(config.endpoints.reschedule, {items: items})
                .done(function(response) {
                    Janitor.Utils.showNotification(
                        'Rescheduled ' + items.length + ' items successfully', 
                        'success'
                    );
                    
                    // Refresh the page or update the table
                    if (response.redirect) {
                        window.location.href = response.redirect;
                    } else {
                        location.reload();
                    }
                })
                .fail(function(xhr) {
                    var message = 'Mass reschedule failed';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                })
                .always(function() {
                    $btn.prop('disabled', false).text(originalText);
                });
        }

        /**
         * Handle reschedule form submission
         */
        function handleRescheduleForm(e) {
            e.preventDefault();
            var $form = $(this);
            var formData = Janitor.Forms.serialize($form[0]);

            Janitor.Ajax.post($form.attr('action') || config.endpoints.reschedule, formData)
                .done(function(response) {
                    Janitor.Utils.showNotification('Items rescheduled successfully', 'success');
                    
                    if (response.redirect) {
                        window.location.href = response.redirect;
                    }
                })
                .fail(function(xhr) {
                    var message = 'Reschedule failed';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                });
        }

        /**
         * Handle kill job action
         */
        function handleKillJob(e) {
            e.preventDefault();
            var $btn = $(this);
            var runId = $btn.data('run-id');

            if (!confirm(config.confirmations.kill)) {
                return;
            }

            var originalText = $btn.text();
            $btn.prop('disabled', true).text('Killing...');

            Janitor.Ajax.post(config.endpoints.kill, {run_id: runId})
                .done(function() {
                    Janitor.Utils.showNotification('Job killed successfully', 'success');
                    
                    // Update the row to show killed status
                    var $row = $btn.closest('tr');
                    $row.find('.status-cell').text('Killed');
                    $btn.remove();
                })
                .fail(function(xhr) {
                    var message = 'Failed to kill job';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                    $btn.prop('disabled', false).text(originalText);
                });
        }

        /**
         * Handle reprocess logs action
         */
        function handleReprocessLogs(e) {
            e.preventDefault();
            var $btn = $(this);
            var runId = $btn.data('run-id');

            if (!confirm(config.confirmations.reprocess)) {
                return;
            }

            var originalText = $btn.text();
            $btn.prop('disabled', true).text('Reprocessing...');

            Janitor.Ajax.post(config.endpoints.reprocess, {run_id: runId})
                .done(function() {
                    Janitor.Utils.showNotification('Log reprocessing initiated', 'success');
                })
                .fail(function(xhr) {
                    var message = 'Failed to reprocess logs';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                })
                .always(function() {
                    $btn.prop('disabled', false).text(originalText);
                });
        }

        /**
         * Handle scan action
         */
        function handleScan(e) {
            e.preventDefault();
            var $btn = $(this);
            var originalText = $btn.text();
            
            $btn.prop('disabled', true).text('Scanning...');

            Janitor.Ajax.post(config.endpoints.scan)
                .done(function(response) {
                    Janitor.Utils.showNotification('Scan initiated successfully', 'success');
                    
                    if (response.count) {
                        Janitor.Utils.showNotification(
                            'Found ' + response.count + ' items to publish', 
                            'info'
                        );
                    }
                })
                .fail(function(xhr) {
                    var message = 'Scan failed';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                })
                .always(function() {
                    $btn.prop('disabled', false).text(originalText);
                });
        }

        /**
         * Handle autopublish action
         */
        function handleAutopublish(e) {
            e.preventDefault();
            var $btn = $(this);
            var originalText = $btn.text();
            
            $btn.prop('disabled', true).text('Publishing...');

            Janitor.Ajax.post(config.endpoints.autopublish)
                .done(function(response) {
                    Janitor.Utils.showNotification('Autopublish initiated successfully', 'success');
                    
                    if (response.count) {
                        Janitor.Utils.showNotification(
                            'Publishing ' + response.count + ' items', 
                            'info'
                        );
                    }
                })
                .fail(function(xhr) {
                    var message = 'Autopublish failed';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                })
                .always(function() {
                    $btn.prop('disabled', false).text(originalText);
                });
        }

        /**
         * Handle select all checkbox
         */
        function handleSelectAll(e) {
            var $checkbox = $(this);
            var $table = $checkbox.closest('table');
            var checked = $checkbox.prop('checked');
            
            $table.find('.item-checkbox').prop('checked', checked);
            updateMassActionButtons();
        }

        /**
         * Update mass action buttons based on selection
         */
        function updateMassActionButtons() {
            var selectedCount = $('.item-checkbox:checked').length;
            var $massActionBtns = $('.mass-action-btn');
            
            if (selectedCount > 0) {
                $massActionBtns.prop('disabled', false);
                $massActionBtns.find('.count').text(selectedCount);
            } else {
                $massActionBtns.prop('disabled', true);
            }
        }

        /**
         * Setup form validation
         */
        function setupFormValidation() {
            // Validate reschedule forms
            $('form.reschedule-form').on('submit', function(e) {
                var $form = $(this);
                var selectedItems = $form.find('.item-checkbox:checked');
                
                if (selectedItems.length === 0) {
                    e.preventDefault();
                    Janitor.Utils.showNotification('No items selected for rescheduling', 'warning');
                    return false;
                }
            });

            // Validate publish forms
            $('form.publish-form').on('submit', function(e) {
                var $form = $(this);
                var action = $form.find('input[name="action"]').val();
                
                if (!action) {
                    e.preventDefault();
                    Janitor.Utils.showNotification('No publish action specified', 'warning');
                    return false;
                }
            });
        }

        /**
         * Initialize buttons and their states
         */
        function initializeButtons() {
            // Initialize tooltips on admin buttons
            $('[data-toggle="tooltip"]').tooltip();
            
            // Update mass action button states
            updateMassActionButtons();
            
            // Set up real-time updates for job status
            if (window.Janitor && window.Janitor.WebSocket) {
                window.Janitor.WebSocket.registerHandler('job_status', function(data) {
                    updateJobStatus(data);
                });
            }
        }

        /**
         * Update job status in real-time
         */
        function updateJobStatus(data) {
            if (data.run_id) {
                var $row = $('tr[data-run-id="' + data.run_id + '"]');
                if ($row.length > 0) {
                    $row.find('.status-cell').text(data.status);
                    
                    if (data.status === 'completed' || data.status === 'failed') {
                        $row.find('.kill-job-btn').remove();
                    }
                }
            }
        }

        /**
         * Refresh admin data
         */
        function refresh() {
            location.reload();
        }

        // Initialize when DOM is ready
        $(document).ready(function() {
            init();
        });

        // Public API
        return {
            init: init,
            refresh: refresh,
            config: config
        };
    })();

})(window, document, jQuery);