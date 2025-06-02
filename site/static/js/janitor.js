/**
 * Janitor Web Application JavaScript Framework
 * Enhanced for real-time functionality, AJAX operations, and modern web interactions
 */

(function(window, document, $) {
    'use strict';

    // Namespace for Janitor functionality
    window.Janitor = window.Janitor || {};

    /**
     * WebSocket Real-time Communication Module
     */
    Janitor.WebSocket = (function() {
        var handlers = [];
        var connection = null;
        var reconnectAttempts = 0;
        var maxReconnectAttempts = 5;
        var reconnectDelay = 1000; // Start with 1 second

        function getWebSocketUrl() {
            var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
            var host = location.hostname;
            var port = location.port ? ':' + location.port : '';
            return protocol + '//' + host + port + '/ws/notifications';
        }

        function connect() {
            var wsUrl = getWebSocketUrl();
            if (!wsUrl) {
                console.error('Unable to determine WebSocket URL');
                return;
            }

            try {
                connection = new WebSocket(wsUrl);
                
                connection.onopen = function() {
                    console.log('WebSocket connected');
                    reconnectAttempts = 0;
                    reconnectDelay = 1000;
                };

                connection.onmessage = function(e) {
                    try {
                        var data = JSON.parse(e.data);
                        if (Array.isArray(data) && data.length >= 2) {
                            var messageType = data[0];
                            var payload = data[1];
                            
                            handlers.forEach(function(handler) {
                                if (handler.kind === messageType) {
                                    handler.callback(payload);
                                }
                            });
                            
                            console.log('WebSocket message:', data);
                        }
                    } catch (error) {
                        console.error('Error parsing WebSocket message:', error);
                    }
                };

                connection.onerror = function(error) {
                    console.error('WebSocket error:', error);
                };

                connection.onclose = function(event) {
                    console.log('WebSocket connection closed:', event.code, event.reason);
                    connection = null;
                    
                    // Attempt to reconnect if not a clean close
                    if (!event.wasClean && reconnectAttempts < maxReconnectAttempts) {
                        setTimeout(function() {
                            reconnectAttempts++;
                            reconnectDelay *= 2; // Exponential backoff
                            console.log('Attempting to reconnect... (attempt ' + reconnectAttempts + ')');
                            connect();
                        }, reconnectDelay);
                    }
                };
            } catch (error) {
                console.error('Failed to create WebSocket connection:', error);
            }
        }

        function registerHandler(kind, callback) {
            if (typeof kind !== 'string' || typeof callback !== 'function') {
                console.error('Invalid handler registration:', kind, callback);
                return;
            }
            handlers.push({kind: kind, callback: callback});
        }

        function disconnect() {
            if (connection && connection.readyState === WebSocket.OPEN) {
                connection.close(1000, 'Page unloading');
            }
        }

        function isConnected() {
            return connection && connection.readyState === WebSocket.OPEN;
        }

        // Initialize WebSocket on page load
        $(document).ready(function() {
            connect();
        });

        // Clean up on page unload
        $(window).on('beforeunload', function() {
            disconnect();
        });

        return {
            registerHandler: registerHandler,
            isConnected: isConnected,
            disconnect: disconnect,
            reconnect: connect
        };
    })();

    /**
     * AJAX Utilities Module
     */
    Janitor.Ajax = (function() {
        // Default AJAX settings
        var defaultSettings = {
            timeout: 30000,
            dataType: 'json',
            headers: {
                'X-Requested-With': 'XMLHttpRequest'
            }
        };

        function request(url, options) {
            var settings = $.extend({}, defaultSettings, options);
            
            return $.ajax(url, settings)
                .fail(function(xhr, status, error) {
                    console.error('AJAX request failed:', {
                        url: url,
                        status: xhr.status,
                        statusText: xhr.statusText,
                        error: error
                    });
                    
                    // Handle authentication errors
                    if (xhr.status === 401) {
                        window.location.href = '/login?next=' + encodeURIComponent(window.location.pathname);
                    }
                });
        }

        function get(url, data, options) {
            return request(url, $.extend({type: 'GET', data: data}, options));
        }

        function post(url, data, options) {
            return request(url, $.extend({type: 'POST', data: data}, options));
        }

        function put(url, data, options) {
            return request(url, $.extend({type: 'PUT', data: data}, options));
        }

        function del(url, data, options) {
            return request(url, $.extend({type: 'DELETE', data: data}, options));
        }

        return {
            request: request,
            get: get,
            post: post,
            put: put,
            delete: del
        };
    })();

    /**
     * Form Handling Module
     */
    Janitor.Forms = (function() {
        function serialize(form) {
            var formData = {};
            var array = $(form).serializeArray();
            
            $.each(array, function(i, field) {
                if (formData[field.name]) {
                    if (!Array.isArray(formData[field.name])) {
                        formData[field.name] = [formData[field.name]];
                    }
                    formData[field.name].push(field.value);
                } else {
                    formData[field.name] = field.value;
                }
            });
            
            return formData;
        }

        function submitWithAjax(form, options) {
            var $form = $(form);
            var url = $form.attr('action') || window.location.pathname;
            var method = $form.attr('method') || 'POST';
            var data = serialize(form);
            
            return Janitor.Ajax.request(url, $.extend({
                type: method,
                data: data
            }, options));
        }

        function enableAutoSubmit(selector) {
            $(document).on('change', selector, function() {
                var $form = $(this).closest('form');
                if ($form.length) {
                    $form.submit();
                }
            });
        }

        return {
            serialize: serialize,
            submitWithAjax: submitWithAjax,
            enableAutoSubmit: enableAutoSubmit
        };
    })();

    /**
     * Utility Functions Module
     */
    Janitor.Utils = (function() {
        // Please keep this logic in sync with the Rust backend duration formatting
        function formatDuration(seconds) {
            if (!moment || typeof seconds !== 'number') {
                return seconds + 's';
            }
            
            var d = moment.duration(seconds, 'seconds');
            
            if (d.weeks() > 0) {
                return d.weeks() + 'w' + (d.days() % 7) + 'd';
            }
            if (d.days() > 0) {
                return d.days() + 'd' + (d.hours() % 24) + 'h';
            }
            if (d.hours() > 0) {
                return d.hours() + 'h' + (d.minutes() % 60) + 'm';
            }
            if (d.minutes() > 0) {
                return d.minutes() + 'm' + (d.seconds() % 60) + 's';
            }
            return d.seconds() + 's';
        }

        function escapeHtml(text) {
            var div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        function showNotification(message, type) {
            type = type || 'info';
            
            // Create notification element
            var $notification = $('<div>')
                .addClass('notification notification-' + type)
                .text(message)
                .css({
                    position: 'fixed',
                    top: '20px',
                    right: '20px',
                    padding: '10px 15px',
                    borderRadius: '4px',
                    zIndex: 9999,
                    opacity: 0
                });
            
            // Add type-specific styling
            var colors = {
                success: {background: '#d4edda', color: '#155724', border: '#c3e6cb'},
                error: {background: '#f8d7da', color: '#721c24', border: '#f5c6cb'},
                warning: {background: '#fff3cd', color: '#856404', border: '#ffeaa7'},
                info: {background: '#d1ecf1', color: '#0c5460', border: '#bee5eb'}
            };
            
            if (colors[type]) {
                $notification.css(colors[type]);
            }
            
            $('body').append($notification);
            
            // Animate in
            $notification.animate({opacity: 1}, 300);
            
            // Auto-remove after 5 seconds
            setTimeout(function() {
                $notification.animate({opacity: 0}, 300, function() {
                    $notification.remove();
                });
            }, 5000);
        }

        return {
            formatDuration: formatDuration,
            escapeHtml: escapeHtml,
            showNotification: showNotification
        };
    })();

    /**
     * Chart Configuration and Colors
     */
    window.chartColors = {
        red: 'rgb(255, 99, 132)',
        orange: 'rgb(255, 159, 64)',
        yellow: 'rgb(255, 205, 86)',
        green: 'rgb(75, 192, 192)',
        blue: 'rgb(54, 162, 235)',
        purple: 'rgb(153, 102, 255)',
        grey: 'rgb(201, 203, 207)',
        lightRed: 'rgba(255, 99, 132, 0.6)',
        lightOrange: 'rgba(255, 159, 64, 0.6)',
        lightYellow: 'rgba(255, 205, 86, 0.6)',
        lightGreen: 'rgba(75, 192, 192, 0.6)',
        lightBlue: 'rgba(54, 162, 235, 0.6)',
        lightPurple: 'rgba(153, 102, 255, 0.6)',
        lightGrey: 'rgba(201, 203, 207, 0.6)'
    };

    // Backward compatibility - expose functions globally
    window.registerHandler = Janitor.WebSocket.registerHandler;
    window.format_duration = Janitor.Utils.formatDuration;

})(window, document, jQuery);
