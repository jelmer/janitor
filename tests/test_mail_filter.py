#!/usr/bin/python
# Copyright (C) 2022 Jelmer Vernooij <jelmer@jelmer.uk>
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program; if not, write to the Free Software
# Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA

from io import BytesIO

from janitor.mail_filter import parse_email


def test_parse_github_merged_email():
    assert "https://github.com/UbuntuBudgie/budgie-desktop/pull/78" == parse_email(BytesIO(b"""\
From noreply@github.com Fri Nov  4 13:18:36 2022
Return-Path: <noreply@github.com>
Delivered-To: jelmer+janitor@jelmer.uk
Date: Fri, 04 Nov 2022 06:18:29 -0700
From: Some User <notifications@github.com>
Reply-To: UbuntuBudgie/budgie-desktop <reply+AKT4UUMF3LHYRIQTBKZCJRGBOJB2LEVBNHHFICBOZM@reply.github.com>
To: UbuntuBudgie/budgie-desktop <budgie-desktop@noreply.github.com>
Cc: Debian Janitor <janitor@jelmer.uk>, 
 Author <author@noreply.github.com>
Message-ID: <UbuntuBudgie/budgie-desktop/pull/78/issue_event/7740657652@github.com>
In-Reply-To: <UbuntuBudgie/budgie-desktop/pull/78@github.com>
References: <UbuntuBudgie/budgie-desktop/pull/78@github.com>
Subject: Re: [UbuntuBudgie/budgie-desktop] Apply hints suggested by the
 multi-arch hinter (PR #78)
Mime-Version: 1.0
Content-Type: multipart/alternative;
 boundary="--==_mimepart_63651125482a9_93f6d55c472112";
 charset=UTF-8
Content-Transfer-Encoding: 7bit
Precedence: list
X-GitHub-Sender: fossfreedom
X-GitHub-Recipient: debian-janitor
X-GitHub-Reason: author
List-ID: UbuntuBudgie/budgie-desktop <budgie-desktop.UbuntuBudgie.github.com>
List-Archive: https://github.com/UbuntuBudgie/budgie-desktop
List-Post: <mailto:reply+AKT4UUMF3LHYRIQTBKZCJRGBOJB2LEVBNHHFICBOZM@reply.github.com>
List-Unsubscribe: <mailto:unsub+AKT4UUMF3LHYRIQTBKZCJRGBOJB2LEVBNHHFICBOZM@reply.github.com>,
 <https://github.com/notifications/unsubscribe/AKT4UUKTFRGZK3G6E55BSBLWGUEKLANCNFSM6AAAAAARFRWCMY>
X-Auto-Response-Suppress: All
X-GitHub-Recipient-Address: janitor@jelmer.uk
Status: RO
Content-Length: 2470
Lines: 42


----==_mimepart_63651125482a9_93f6d55c472112
Content-Type: text/plain;
 charset=UTF-8
Content-Transfer-Encoding: 7bit

Merged #78 into debian.

-- 
Reply to this email directly or view it on GitHub:
https://github.com/UbuntuBudgie/budgie-desktop/pull/78#event-7740657652
You are receiving this because you authored the thread.

Message ID: <UbuntuBudgie/budgie-desktop/pull/78/issue_event/7740657652@github.com>
----==_mimepart_63651125482a9_93f6d55c472112
Content-Type: text/html;
 charset=UTF-8
Content-Transfer-Encoding: 7bit

<p></p>
<p dir="auto">Merged <a class="issue-link js-issue-link" data-error-text="Failed to load title" data-id="1409822411" data-permission-text="Title is private" data-url="https://github.com/UbuntuBudgie/budgie-desktop/issues/78" data-hovercard-type="pull_request" data-hovercard-url="/UbuntuBudgie/budgie-desktop/pull/78/hovercard" href="https://github.com/UbuntuBudgie/budgie-desktop/pull/78">#78</a> into debian.</p>

<p style="font-size:small;-webkit-text-size-adjust:none;color:#666;">&mdash;<br />Reply to this email directly, <a href="https://github.com/UbuntuBudgie/budgie-desktop/pull/78#event-7740657652">view it on GitHub</a>, or <a href="https://github.com/notifications/unsubscribe-auth/AKT4UUL4O2DVXUA5C4LJJSTWGUEKLANCNFSM6AAAAAARFRWCMY">unsubscribe</a>.<br />You are receiving this because you authored the thread.<img src="https://github.com/notifications/beacon/AKT4UUP22SNQCQEFA27TRJLWGUEKLA5CNFSM6AAAAAARFRWCM2WGG33NNVSW45C7OR4XAZNWJFZXG5LFIV3GK3TUJZXXI2LGNFRWC5DJN5XKUY3PNVWWK3TUL5UWJTYAAAAADTLBB72A.gif" height="1" width="1" alt="" /><span style="color: transparent; font-size: 0; display: none; visibility: hidden; overflow: hidden; opacity: 0; width: 0; height: 0; max-width: 0; max-height: 0; mso-hide: all">Message ID: <span>&lt;UbuntuBudgie/budgie-desktop/pull/78/issue_event/7740657652</span><span>@</span><span>github</span><span>.</span><span>com&gt;</span></span></p>
<script type="application/ld+json">[
{
"@context": "http://schema.org",
"@type": "EmailMessage",
"potentialAction": {
"@type": "ViewAction",
"target": "https://github.com/UbuntuBudgie/budgie-desktop/pull/78#event-7740657652",
"url": "https://github.com/UbuntuBudgie/budgie-desktop/pull/78#event-7740657652",
"name": "View Pull Request"
},
"description": "View this Pull Request on GitHub",
"publisher": {
"@type": "Organization",
"name": "GitHub",
"url": "https://github.com"
}
}
]</script>
----==_mimepart_63651125482a9_93f6d55c472112--
"""))


def test_parse_gitlab_merged_email():
    assert "https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2" == parse_email(BytesIO(b"""\
From gitlab@salsa.debian.org Fri Nov  4 14:35:04 2022
Return-Path: <gitlab@salsa.debian.org>
Delivered-To: jelmer+janitor@jelmer.uk
Date: Fri, 04 Nov 2022 14:34:54 +0000
From: =?UTF-8?B?SmVsbWVyIFZlcm5vb8SzIChAamVsbWVyKQ==?= <gitlab@salsa.debian.org>
Reply-To: Debian / pkg-lojban-common <gitlab+ea8cd99546cbc76a70a527bf26e6eeeb@salsa.debian.org>
To: janitor@jelmer.uk
Message-ID: <3ae5e0b4e2311c797875a0053c78bd28@salsa.debian.org>
In-Reply-To: <merge_request_52492@salsa.debian.org>
References: <reply-ea8cd99546cbc76a70a527bf26e6eeeb@salsa.debian.org>
 <merge_request_52492@salsa.debian.org>
Subject: Re: pkg-lojban-common | Fix some issues reported by lintian (!2)
Mime-Version: 1.0
Content-Type: multipart/alternative;
 boundary="--==_mimepart_6365230e28ca5_49411dbd4c195215f";
 charset=UTF-8
Content-Transfer-Encoding: 7bit
X-GitLab-Project: pkg-lojban-common
X-GitLab-Project-Id: 21620
X-GitLab-Project-Path: debian/pkg-lojban-common
List-Id: debian/pkg-lojban-common
 <21620.pkg-lojban-common.debian.salsa.debian.org>
List-Unsubscribe: <https://salsa.debian.org/-/sent_notifications/ea8cd99546cbc76a70a527bf26e6eeeb/unsubscribe?force=true>,<mailto:gitlab+ea8cd99546cbc76a70a527bf26e6eeeb-unsubscribe@salsa.debian.org>
X-GitLab-MergeRequest-ID: 52492
X-GitLab-MergeRequest-IID: 2
X-GitLab-NotificationReason: 
X-GitLab-Reply-Key: ea8cd99546cbc76a70a527bf26e6eeeb
Auto-Submitted: auto-generated
Status: RO
Content-Length: 3283
Lines: 80


----==_mimepart_6365230e28ca5_49411dbd4c195215f
Content-Type: text/plain;
 charset=UTF-8
Content-Transfer-Encoding: 7bit



Merge request !2 was merged
Merge request URL: https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2
Project:Branches: janitor-team/proposed/pkg-lojban-common:lintian-fixes to debian/pkg-lojban-common:main
Author: Janitor

-- 
Reply to this email directly or view it on GitLab: https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2
You're receiving this email because of your account on salsa.debian.org.



----==_mimepart_6365230e28ca5_49411dbd4c195215f
Content-Type: text/html;
 charset=UTF-8
Content-Transfer-Encoding: 7bit

<!DOCTYPE html PUBLIC "-//W3C//DTD HTML 4.0 Transitional//EN" "http://www.w3.org/TR/REC-html40/loose.dtd">
<html lang="en">
<head>
<meta content="text/html; charset=US-ASCII" http-equiv="Content-Type">
<title>
GitLab
</title>

<style data-premailer="ignore" type="text/css">
a { color: #1068bf; }
</style>

<style>img {
max-width: 100%; height: auto;
}
body {
font-size: 0.875rem;
}
body {
-webkit-text-shadow: rgba(255,255,255,0.01) 0 0 1px;
}
body {
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Noto Sans", Ubuntu, Cantarell, "Helvetica Neue", sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji"; font-size: inherit;
}
</style>
</head>
<body style='font-size: inherit; -webkit-text-shadow: rgba(255,255,255,0.01) 0 0 1px; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Noto Sans", Ubuntu, Cantarell, "Helvetica Neue", sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";'>
<div class="content">

<p>
Merge request <a href="https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2" style="color: #1068bf;">!2</a> was merged
</p>
<p>
Project:Branches: janitor-team/proposed/pkg-lojban-common:lintian-fixes to debian/pkg-lojban-common:main
</p>
<div>
Author: Janitor
</div>

</div>
<div class="footer" style="margin-top: 10px;">
<p style="font-size: small; color: #666;">
&#8212;
<br>
Reply to this email directly or <a href="https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2" style="color: #1068bf;">view it on GitLab</a>.
<br>
You're receiving this email because of your account on <a target="_blank" rel="noopener noreferrer" href="https://salsa.debian.org" style="color: #1068bf;">salsa.debian.org</a>. <a href="https://salsa.debian.org/-/sent_notifications/ea8cd99546cbc76a70a527bf26e6eeeb/unsubscribe" target="_blank" rel="noopener noreferrer" style="color: #1068bf;">Unsubscribe</a> from this thread &#183; <a href="https://salsa.debian.org/-/profile/notifications" target="_blank" rel="noopener noreferrer" class="mng-notif-link" style="color: #1068bf;">Manage all notifications</a> &#183; <a href="https://salsa.debian.org/help" target="_blank" rel="noopener noreferrer" class="help-link" style="color: #1068bf;">Help</a>
<script type="application/ld+json">{"@context":"http://schema.org","@type":"EmailMessage","action":{"@type":"ViewAction","name":"View Merge request","url":"https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2"}}</script>


</p>
</div>
</body>
</html>

----==_mimepart_6365230e28ca5_49411dbd4c195215f--

"""))
