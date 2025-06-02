/**
 * Review System JavaScript for Janitor
 * Handles review queue management, preloading, and review submission
 */

(function(window, document, $) {
    'use strict';

    // Ensure Janitor namespace exists
    window.Janitor = window.Janitor || {};

    /**
     * Review Module
     */
    Janitor.Review = (function() {
        
        // Review queue state
        var reviewQueue = {
            current: null,
            preloadCache: new Map(),
            position: 0,
            total: 0,
            filters: {}
        };

        // Configuration
        var config = {
            preloadCount: 3, // Number of items to preload ahead
            cacheSize: 10,   // Maximum cached items
            endpoints: {
                queue: '/api/review/queue',
                item: '/api/review/item/',
                submit: '/api/review/submit',
                skip: '/api/review/skip'
            }
        };

        /**
         * Initialize review system
         */
        function init() {
            loadReviewQueue();
            attachEventHandlers();
            setupKeyboardShortcuts();
            
            // Set up real-time updates
            if (window.Janitor && window.Janitor.WebSocket) {
                window.Janitor.WebSocket.registerHandler('review_update', handleReviewUpdate);
            }
        }

        /**
         * Load initial review queue
         */
        function loadReviewQueue() {
            var filters = getActiveFilters();
            
            Janitor.Ajax.get(config.endpoints.queue, filters)
                .done(function(response) {
                    reviewQueue.total = response.total || 0;
                    reviewQueue.position = response.position || 0;
                    
                    if (response.items && response.items.length > 0) {
                        reviewQueue.current = response.items[0];
                        loadReviewItem(reviewQueue.current.id);
                        preloadNextItems(response.items.slice(1));
                    } else {
                        showEmptyQueue();
                    }
                    
                    updateQueueStatus();
                })
                .fail(function() {
                    Janitor.Utils.showNotification('Failed to load review queue', 'error');
                });
        }

        /**
         * Load a specific review item
         */
        function loadReviewItem(itemId, callback) {
            // Check cache first
            if (reviewQueue.preloadCache.has(itemId)) {
                var cachedItem = reviewQueue.preloadCache.get(itemId);
                displayReviewItem(cachedItem);
                if (callback) callback(cachedItem);
                return;
            }

            // Load from server
            Janitor.Ajax.get(config.endpoints.item + itemId)
                .done(function(item) {
                    // Cache the item
                    reviewQueue.preloadCache.set(itemId, item);
                    
                    // Display if it's the current item
                    if (reviewQueue.current && reviewQueue.current.id === itemId) {
                        displayReviewItem(item);
                    }
                    
                    if (callback) callback(item);
                })
                .fail(function(xhr) {
                    if (xhr.status === 401) {
                        window.location.href = '/login?next=' + encodeURIComponent(window.location.pathname);
                    } else {
                        Janitor.Utils.showNotification('Failed to load review item', 'error');
                    }
                });
        }

        /**
         * Display review item in the UI
         */
        function displayReviewItem(item) {
            var $container = $('#review-content');
            if ($container.length === 0) {
                console.error('Review content container not found');
                return;
            }

            try {
                // Update item details
                $('#review-package-name').text(item.package || item.codebase || '');
                $('#review-campaign').text(item.campaign || '');
                $('#review-description').html(item.description || '');
                
                // Update diff or changes
                if (item.diff_content) {
                    $('#review-diff').html(item.diff_content);
                }
                
                // Update build logs if available
                if (item.build_log_url) {
                    $('#review-build-log').attr('href', item.build_log_url);
                }
                
                // Update merge proposal link
                if (item.merge_proposal_url) {
                    $('#review-merge-proposal').attr('href', item.merge_proposal_url).show();
                } else {
                    $('#review-merge-proposal').hide();
                }
                
                // Update review form
                $('#review-form input[name="item_id"]').val(item.id);
                
                // Reset form state
                resetReviewForm();
                
                // Update navigation
                updateQueueStatus();
                
            } catch (error) {
                console.error('Error displaying review item:', error);
                Janitor.Utils.showNotification('Error displaying review item', 'error');
            }
        }

        /**
         * Preload next items in the queue
         */
        function preloadNextItems(items) {
            items.slice(0, config.preloadCount).forEach(function(item) {
                if (!reviewQueue.preloadCache.has(item.id)) {
                    loadReviewItem(item.id);
                }
            });
        }

        /**
         * Submit review verdict
         */
        function submitReview(verdict, comment) {
            if (!reviewQueue.current) {
                Janitor.Utils.showNotification('No item to review', 'error');
                return;
            }

            var data = {
                item_id: reviewQueue.current.id,
                verdict: verdict,
                comment: comment || ''
            };

            // Disable form during submission
            $('#review-form button').prop('disabled', true);
            
            Janitor.Ajax.post(config.endpoints.submit, data)
                .done(function(response) {
                    Janitor.Utils.showNotification('Review submitted successfully', 'success');
                    
                    // Move to next item
                    if (response.next_item) {
                        reviewQueue.current = response.next_item;
                        reviewQueue.position++;
                        loadReviewItem(response.next_item.id);
                    } else {
                        // Queue is empty
                        showEmptyQueue();
                    }
                })
                .fail(function(xhr) {
                    var message = 'Failed to submit review';
                    if (xhr.responseJSON && xhr.responseJSON.error) {
                        message = xhr.responseJSON.error;
                    }
                    Janitor.Utils.showNotification(message, 'error');
                })
                .always(function() {
                    $('#review-form button').prop('disabled', false);
                });
        }

        /**
         * Skip current review item
         */
        function skipReview() {
            if (!reviewQueue.current) {
                return;
            }

            Janitor.Ajax.post(config.endpoints.skip, {item_id: reviewQueue.current.id})
                .done(function(response) {
                    if (response.next_item) {
                        reviewQueue.current = response.next_item;
                        loadReviewItem(response.next_item.id);
                    } else {
                        showEmptyQueue();
                    }
                })
                .fail(function() {
                    Janitor.Utils.showNotification('Failed to skip item', 'error');
                });
        }

        /**
         * Show empty queue message
         */
        function showEmptyQueue() {
            $('#review-content').html(
                '<div class="alert alert-info">' +
                '<h4>Review Queue Empty</h4>' +
                '<p>There are no items in the review queue at the moment.</p>' +
                '<p><a href="#" onclick="location.reload()">Refresh</a> to check for new items.</p>' +
                '</div>'
            );
            reviewQueue.current = null;
            updateQueueStatus();
        }

        /**
         * Update queue status display
         */
        function updateQueueStatus() {
            var $status = $('#review-queue-status');
            if ($status.length === 0) return;

            if (reviewQueue.current) {
                $status.text('Item ' + (reviewQueue.position + 1) + ' of ' + reviewQueue.total);
            } else {
                $status.text('Queue empty');
            }
        }

        /**
         * Get active filters from the page
         */
        function getActiveFilters() {
            var filters = {};
            
            $('input[name^="filter_"]:checked').each(function() {
                var filterName = $(this).attr('name').replace('filter_', '');
                filters[filterName] = $(this).val();
            });
            
            $('select[name^="filter_"]').each(function() {
                var filterName = $(this).attr('name').replace('filter_', '');
                var value = $(this).val();
                if (value) {
                    filters[filterName] = value;
                }
            });
            
            return filters;
        }

        /**
         * Reset review form to initial state
         */
        function resetReviewForm() {
            $('#review-form')[0].reset();
            $('#review-comment').val('').hide();
            $('#review-form button').prop('disabled', false);
        }

        /**
         * Attach event handlers
         */
        function attachEventHandlers() {
            // Review verdict buttons
            $(document).on('click', '.review-btn', function(e) {
                e.preventDefault();
                var verdict = $(this).data('verdict');
                var requiresComment = $(this).data('requires-comment');
                
                if (requiresComment) {
                    var comment = prompt('Please provide a reason for ' + verdict + ':');
                    if (comment === null) {
                        return; // User cancelled
                    }
                    submitReview(verdict, comment);
                } else {
                    submitReview(verdict);
                }
            });

            // Skip button
            $(document).on('click', '#skip-review-btn', function(e) {
                e.preventDefault();
                skipReview();
            });

            // Filter changes
            $(document).on('change', 'input[name^="filter_"], select[name^="filter_"]', function() {
                reviewQueue.filters = getActiveFilters();
                loadReviewQueue();
            });

            // Review form submission
            $(document).on('submit', '#review-form', function(e) {
                e.preventDefault();
                var formData = Janitor.Forms.serialize(this);
                submitReview(formData.verdict, formData.comment);
            });

            // Reject button with comment
            $(document).on('click', '#reject-review-btn', function(e) {
                e.preventDefault();
                $('#review-comment').show().focus();
            });

            // Comment form handling
            $(document).on('click', '#submit-comment-btn', function(e) {
                e.preventDefault();
                var comment = $('#review-comment').val().trim();
                if (!comment) {
                    alert('Please provide a comment for rejection');
                    return;
                }
                submitReview('reject', comment);
            });
        }

        /**
         * Setup keyboard shortcuts
         */
        function setupKeyboardShortcuts() {
            $(document).on('keydown', function(e) {
                // Only handle shortcuts if not in an input field
                if ($(e.target).is('input, textarea, select')) {
                    return;
                }

                switch (e.key) {
                    case 'a':
                    case 'A':
                        e.preventDefault();
                        submitReview('approve');
                        break;
                    case 'r':
                    case 'R':
                        e.preventDefault();
                        $('#reject-review-btn').click();
                        break;
                    case 's':
                    case 'S':
                        e.preventDefault();
                        skipReview();
                        break;
                    case 'n':
                    case 'N':
                        e.preventDefault();
                        skipReview();
                        break;
                    case 'Escape':
                        $('#review-comment').hide();
                        resetReviewForm();
                        break;
                }
            });
        }

        /**
         * Handle real-time review updates via WebSocket
         */
        function handleReviewUpdate(data) {
            if (data.type === 'queue_update') {
                // Refresh queue if significant changes
                loadReviewQueue();
            } else if (data.type === 'item_update' && reviewQueue.current && data.item_id === reviewQueue.current.id) {
                // Update current item if it changed
                loadReviewItem(data.item_id);
            }
        }

        /**
         * Refresh the current review
         */
        function refresh() {
            if (reviewQueue.current) {
                // Clear cache for current item
                reviewQueue.preloadCache.delete(reviewQueue.current.id);
                loadReviewItem(reviewQueue.current.id);
            } else {
                loadReviewQueue();
            }
        }

        /**
         * Get current review state
         */
        function getState() {
            return {
                current: reviewQueue.current,
                position: reviewQueue.position,
                total: reviewQueue.total,
                filters: reviewQueue.filters
            };
        }

        // Initialize when DOM is ready
        $(document).ready(function() {
            if ($('#review-content').length > 0) {
                init();
            }
        });

        // Public API
        return {
            init: init,
            submitReview: submitReview,
            skipReview: skipReview,
            loadReviewItem: loadReviewItem,
            refresh: refresh,
            getState: getState,
            config: config
        };
    })();

})(window, document, jQuery);